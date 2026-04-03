use osm::{
    coord::{
        convertion,
        geo::{GeoBox, GeoPoint},
        mercator::{MercatorBox, MercatorPoint},
    },
    element::{MercatorNode, MercatorWay, NWR},
};
use reqwest::{Client, StatusCode};
use std::{collections::HashMap, rc::Rc, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

use super::api_server::ServerPool;
use super::cache::{self, Cache};

use crate::{rejection, vec2d::Vec2D};

mod chunk;

// IMPORTANT TODO: LOOK INTO MERCATOR https://en.wikipedia.org/wiki/Mercator_projection
// https://stackoverflow.com/questions/14329691/convert-latitude-longitude-point-to-a-pixels-x-y-on-mercator-projection
//
// Idea:
// Only OSM work in geo coordinates, we use Mercator
//
// Store data chunks in mercator coordinates
// Send chunks to the client,
// Client displays mercator coordinates
//
// Client requests Mercator area
// We slice the area, fill it with saved chunks, request the rest
// Send back chunks representing the area (maybe processed ? but not necessary)
// Client redraws everything with the new data
//
// Bonus points for that approach since:
//  If the client zooms in -> All chunks are already cached
//  If the client zooms out -> Only a couple of chunks are to be requested (1 small chunk groups on each side)
//  If the client moves -> Only a small portion of chunks are to be requested (1 small chunk group on one side (maybe 2 if diagonal))

const USER_AGENT: &str = "Cybermap/0.1.0 (linux; x86_64)";

const API_SERVERS: &[&str] = &[
    "https://lz4.overpass-api.de/api/interpreter",
    "https://z.overpass-api.de/api/interpreter",
    "https://overpass-api.de/api/interpreter",
    // Fallback, we won't use it unless we have no other choice
    "https://overpass.private.coffee/api/interpreter",
];

#[derive(serde::Deserialize)]
struct Query {
    inner: String,
}

pub fn build_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let client = reqwest::Client::new();
    let cache: Arc<RwLock<Box<dyn Cache>>> = Arc::new(RwLock::new(Box::new(cache::DiskCache {
        root_path: "./chunk_cache".into(),
    })));

    let api_pool = ServerPool::new(API_SERVERS, Duration::from_secs(1));

    let rate_limiter_config = warp_rate_limit::RateLimitConfig {
        max_requests: 4,
        window: Duration::from_secs(10),
        retry_after_format: warp_rate_limit::RetryAfterFormat::HttpDate,
        ip_header: "X-Forwarded-For".to_owned(),
    };

    let bx = serde_json::to_string(&GeoBox::from_center_and_size(
        GeoPoint::new(40.730610, 73.935242),
        (1., 1.),
    ))
    .unwrap();
    let mut rb = client.get(format!("http://127.0.0.1:42061/overpass_chunk_api"));
    let rb = rb.query(&[("", &bx)]);
    debug!("{rb:?}");
    // rb.send();

    warp::get()
        .and(warp::path("overpass_chunk_api"))
        .and(warp::path::end())
        // Naïve 'security' to make sure bots won't trigger an api call by spamming random sht
        // .and(warp::filters::header::exact(
        //     "cybermap",
        //     "8b3d00bf-b0cc-4a7d-b389-9c0e9d0688f8",
        // ))
        .and(
            warp::query::query::<Query>()
                .map(|query: Query| serde_json::from_str(&query.inner).unwrap()),
        )
        .and(warp_rate_limit::with_rate_limit(rate_limiter_config))
        .and(warp::any().map(move || client.clone()))
        .and(warp::any().map(move || cache.clone()))
        .and(warp::any().map(move || api_pool.clone()))
        .and_then(handle_request)
        .recover(rejection::method_not_allowed)
        .recover(rejection::missing_header)
        .recover(rejection::invalid_header)
        .recover(rejection::invalid_query)
        .recover(rejection::rate_limit)
        .recover(rejection::proxy)
}

#[derive(Debug)]
pub enum OSMProxyRejection {
    CacheFailure,
    InvalidUserData,
    APIServerFailure,
    APIResponseUnpackingFailed,
    NoServerAvailable,
}

impl warp::reject::Reject for OSMProxyRejection {}

async fn handle_request(
    data_query: GeoBox,
    _rli: warp_rate_limit::RateLimitInfo,
    client: Client,
    cache: Arc<RwLock<Box<dyn Cache>>>,
    api_server_pool: ServerPool,
) -> Result<impl Reply, Rejection> {
    // TODO: Rework that into something more gracefull, and maybe add more than 10km ?
    assert!(data_query.width_km() < 10.);
    assert!(data_query.height_km() < 10.);

    debug!("Got chunk request");

    let mbox = data_query.to_mercator();
    let chunk_rects = split_mercatorbox(&mbox);
    let cache_read = cache.read().await;

    let mut chunks = HashMap::<(usize, usize), chunk::Chunk>::with_capacity(chunk_rects.len());

    // Fetch cached chunks
    for (x, y) in chunk_rects.iter() {
        let chunk_rect = chunk_rects.get(x, y).unwrap();

        let key = serde_json::to_string(chunk_rect).unwrap();
        match cache_read.get(&key) {
            Ok(Some(data)) => {
                let chunk = serde_json::from_str::<chunk::Chunk>(&data).unwrap();
                chunks.insert((x, y), chunk);
            }
            Ok(None) => (), // The chunk is new
            Err(e) => {
                error!("Something has gone wrong while fetching chunks from storage: {e}");
                // i+=1;
                return Ok(warp::reply::with_status(
                    "Something went wrong",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        }
    }
    drop(cache_read);

    // build to-be-requested areas
    let to_request = Vec2D::new_from_vec(
        chunk_rects
            .iter()
            .map(|(x, y)| !chunks.contains_key(&(x, y)))
            .collect::<Vec<_>>(),
        chunk_rects.width(),
        chunk_rects.height(),
    )
    .unwrap();
    let to_be_requested: Vec<MercatorBox> = greedy_chunk(&chunk_rects, to_request.clone()); // I don't like this clone

    let mut new_data: NWR = NWR::default();
    for (i, chunk_rect) in to_be_requested.iter().enumerate() {
        // Would be nice to request a bit more than necessary in order to not miss any data (geo -> mercator error)
        // like 5% ?
        let gbox = chunk_rect.to_geo();
        let api_url = match api_server_pool.find_one_ready().await {
            Some(url) => url,
            None => {
                warn!(
                    "Failed to find a ready api url, skipping new chunk fetch after: {i} requests"
                );
                break;
            }
        };

        // Request data for the chunk
        let data = match request_area_with_url(gbox, api_url) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to request geobox {i}: {gbox:?} due to: {e}");
                continue;
            }
        };

        // todo!("Push new nwr to a 'global' new_data variable(NWR)");
    }
    todo!("above");

    // let new_data: MERCATORNWR ?

    assert_eq!(to_request.len(), chunk_rects.len());
    let mut new_chunks: Vec<(chunk::Chunk, (usize, usize))> =
        Vec::with_capacity(to_be_requested.len()); // x, y
    // todo!("For chunks-that-have-no-data");
    for (x, y) in chunk_rects.iter() {
        if to_request.get(x, y) != Some(&true) {
            // that chunk already has data
            continue;
        }
        let chunk_rect = chunk_rects.get(x, y).unwrap();

        todo!("Fill theses chunks with data using the data fetched by the requests above");
        let chunk = chunk::Chunk {
            zone: *chunk_rect,
            nodes: todo!(),              // Fill from data
            ways: todo!(),               // Fill from data
            non_truncated_ways: todo!(), // Fill from data
        };
        new_chunks.push((chunk, (x, y)));
    }
    todo!("Write the new chunks to disk");

    // Save to cache
    let mut cache_write = cache.write().await;

    for (chunk, pos) in new_chunks.into_iter() {
        let key = serde_json::to_string(&chunk.zone).unwrap();
        let data = serde_json::to_string(&chunk).unwrap();
        cache_write.insert(&key, &data).unwrap();

        // Push to chunks
        chunks.insert(pos, chunk);
    }
    drop(cache_write);

    todo!("Group the chunk grid (now full unless errors) into a big NWR");

    todo!("Send back over the wire");

    Ok(warp::reply::with_status("", StatusCode::IM_A_TEAPOT))
}

fn request_area_with_url(gbox: GeoBox, api_url: Arc<str>) -> Result<NWR, String> {
    debug!("Request area: {gbox:#?} at {api_url}");
    Err(String::from("test"))
}

fn greedy_chunk(
    chunk_rects: &Vec2D<MercatorBox>,
    mut chunks_to_request: Vec2D<bool>,
) -> Vec<MercatorBox> {
    assert_eq!(chunk_rects.len(), chunks_to_request.len());
    let width = chunk_rects.width();
    let height = chunk_rects.height();

    // let xy_to_index = |x: u16, y: u16| -> usize { (y * width as u16 + x) as usize };

    let mut boxes_to_request = Vec::<MercatorBox>::new();

    loop {
        debug!("Searching for starting point in: {chunks_to_request:?}");
        // find a starting point
        //
        // Theses are inclusive
        let mut start_x = 0;
        let mut start_y = 0;
        // This iterator checks bounds, there is no way it creates a none, so the exit condition cannot be met
        for (x, y) in chunks_to_request.iter() {
            match chunks_to_request.get(x, y) {
                Some(true) => {
                    start_x = x;
                    start_y = y;
                    break;
                }
                Some(false) => continue,
                None => unreachable!(), // Since it's a bound-aware iterator, we cannot get invalid xy
            }
        }
        // So we gotta check for it here
        if start_x == 0 && chunks_to_request.get(start_x, start_y) == Some(&false) {
            debug!("Boxes to request: {boxes_to_request:?}");
            return boxes_to_request;
        } else {
            // Don't forget to set it to false, AFTER checking for emptyness
            //
            // This is not ideal, but it's fine for now
            chunks_to_request.set(start_x, start_y, false).unwrap();
        }

        debug!("Found start point in: {start_x}, {start_y}");

        debug!("Searching for end point in: {chunks_to_request:?}");
        // Find an end point
        //
        // Inclusive too
        let (end_x, end_y) = {
            let mut end_x = start_x;
            let mut end_y = start_y;

            loop {
                let current_x = end_x + 1;

                match chunks_to_request.get(current_x, end_y) {
                    Some(false) | None => {
                        break;
                    }
                    Some(true) => {
                        // Very important to disable any chunk already searched
                        chunks_to_request.set(current_x, end_y, false).unwrap();
                        end_x = current_x;
                        continue;
                    }
                }
            }

            assert!(chunks_to_request.get(end_x, end_y).is_some());

            loop {
                let current_y = end_y + 1;

                // If there is any chunk not marked for request in the new row, abort
                if (start_x..=end_x).any(|x| {
                    debug!(
                        "[Y] Checking for {x}, {current_y}: {:?}",
                        chunks_to_request.get(x, current_y)
                    );

                    chunks_to_request.get(x, current_y) != Some(&true)
                }) {
                    break;
                }

                // Very important to disable any chunk already searched
                (start_x..=end_x).for_each(|x| {
                    chunks_to_request.set(x, current_y, false).unwrap();
                });

                end_y = current_y;
            }
            debug!("Found start point in: {start_x}, {start_y}");

            (end_x, end_y)
        };
        debug!("Found end point in: {end_x}, {end_y}");

        let start = chunk_rects.get(start_x, start_y).unwrap();
        let end = chunk_rects.get(end_x, end_y).unwrap();

        boxes_to_request.push(MercatorBox::new(start.topleft(), end.botright()));
    }
}

fn round(f: f64, decimals: u32) -> f64 {
    let shift_factor = 10_f64.powi(decimals as i32);

    (f * shift_factor).round() / shift_factor
}

// A small doubt I have is, I've seen in the visualisation that chunks converted to geocoordinates, overlap or miss zones
// Which make sense since geocoordinates are on a sphere,
//
// And here, since we use mercator for the splitting, it *should* be fine, at least for splitting data in chunks
//
// A tricky thing is gonna be roads, since some nodes are gonna be outside zones.
// And same for joining chunks back together, ways and their nodes are gonna need to be checked for uniqueness
fn split_oms_data_in_chunks(geobox: &GeoBox, nwr: osm::element::NWR) -> Vec2D<chunk::Chunk> {
    let mbox = geobox.to_mercator();
    let chunk_rects = split_mercatorbox(&mbox);

    let mut chunks = Vec::with_capacity(chunk_rects.len());

    let nodes = nwr
        .nodes
        .into_iter()
        .map(|(id, node)| {
            (
                id,
                MercatorNode {
                    osm_id: node.osm_id,
                    pos: convertion::geo_to_mercator(&node.pos),
                    tags: node.tags,
                },
            )
        })
        .collect::<HashMap<u64, MercatorNode>>();

    let ways = nwr
        .ways
        .into_iter()
        .map(|(id, way)| {
            (
                id,
                MercatorWay {
                    osm_id: way.osm_id,
                    nodes: way
                        .nodes
                        .iter()
                        .map(|node| MercatorNode {
                            osm_id: node.osm_id,
                            pos: convertion::geo_to_mercator(&node.pos),
                            tags: node.tags.clone(),
                        })
                        .collect::<Arc<[MercatorNode]>>(),
                    tags: way.tags.clone(), // Somewhat cheap since they are Rc<str>
                },
            )
        })
        .collect::<HashMap<u64, MercatorWay>>();

    for (x, y) in chunk_rects.iter() {
        let chunk_rect = chunk_rects.get(x, y).unwrap();

        let contains_node = |node: &MercatorNode| -> bool {
            let x1 = chunk_rect.min().x();
            let y1 = chunk_rect.max().y();
            let x2 = chunk_rect.max().x();
            let y2 = chunk_rect.min().y();

            x1 < node.pos.x() && node.pos.x() < x2 && y1 < node.pos.y() && node.pos.y() < y2
        };

        let chunk_nodes = nodes
            .iter()
            .filter(|(_id, node)| contains_node(node))
            .map(|(id, node)| (*id, node.clone()))
            .collect::<HashMap<u64, MercatorNode>>();
        let chunk_ways = ways
            .clone()
            .into_iter()
            .flat_map(|(id, mut way)| {
                way.nodes = way
                    .nodes
                    .iter()
                    .filter(|node| contains_node(node))
                    .cloned()
                    .collect::<Arc<[MercatorNode]>>();

                if way.nodes.is_empty() {
                    None
                } else {
                    Some((id, way))
                }
            })
            .collect::<HashMap<u64, MercatorWay>>();

        let chunk_full_ways = chunk_ways
            .keys()
            .map(|id| ways.get(id).unwrap()) // this unwrap should NEVER fail as chunk_ways is a subset of ways
            .map(|way| {
                (
                    way.osm_id,
                    way.nodes
                        .iter()
                        .map(|node| node.osm_id)
                        .collect::<Vec<u64>>(),
                )
            })
            .collect::<HashMap<u64, Vec<u64>>>();

        let chunk = chunk::Chunk {
            zone: *chunk_rect,
            nodes: chunk_nodes,
            ways: chunk_ways,
            non_truncated_ways: chunk_full_ways,
        };

        chunks.push(chunk);
    }

    Vec2D::new_from_vec(chunks, chunk_rects.width(), chunk_rects.height()).unwrap()
}

fn split_geobox(geobox: &GeoBox) -> Vec<GeoBox> {
    let mut chunks = Vec::new();
    const CHUNK_SIZE: f64 = 0.005; // degrees
    let min_lat = geobox.min().lat();
    let max_lat = geobox.max().lat();
    let min_lon = geobox.min().lon();
    let max_lon = geobox.max().lon();

    let start_lat = round((min_lat / CHUNK_SIZE).floor() * CHUNK_SIZE, 10);
    let end_lat = round((max_lat / CHUNK_SIZE).ceil() * CHUNK_SIZE, 10);
    let start_lon = round((min_lon / CHUNK_SIZE).floor() * CHUNK_SIZE, 10);
    let end_lon = round((max_lon / CHUNK_SIZE).ceil() * CHUNK_SIZE, 10);

    debug!("{geobox:?}");
    debug!("{start_lat}, {start_lon}, {end_lat}, {end_lon}");

    let mut lat = start_lat;
    while lat < end_lat {
        let mut lon = start_lon;
        while lon < end_lon {
            let chunk_min = GeoPoint::new(lat, lon);
            let chunk_max = GeoPoint::new(round(lat + CHUNK_SIZE, 10), round(lon + CHUNK_SIZE, 10));
            // debug!("Making chunk: {chunk_min:?}, {chunk_max:?}");
            let chunk = GeoBox::new(chunk_min, chunk_max);
            chunks.push(chunk);
            lon += CHUNK_SIZE;
            lon = round(lon, 4);
        }
        lat += CHUNK_SIZE;
        lat = round(lat, 4);
        println!("{}", chunks.len());
    }

    chunks
}

fn split_mercatorbox(mbox: &MercatorBox) -> Vec2D<MercatorBox> {
    let mut chunks = Vec::new();
    const CHUNK_SIZE: f64 = 100.;
    let min_y = mbox.min().y();
    let max_y = mbox.max().y();
    let min_x = mbox.min().x();
    let max_x = mbox.max().x();

    let start_y = round((min_y / CHUNK_SIZE).floor() * CHUNK_SIZE, 10);
    let end_y = round((max_y / CHUNK_SIZE).ceil() * CHUNK_SIZE, 10);
    let start_x = round((min_x / CHUNK_SIZE).floor() * CHUNK_SIZE, 10);
    let end_x = round((max_x / CHUNK_SIZE).ceil() * CHUNK_SIZE, 10);

    let width = ((end_x - start_x) / CHUNK_SIZE) as usize;
    let height = ((end_y - start_y) / CHUNK_SIZE) as usize;

    debug!("{mbox:?}");
    debug!("{start_y}, {start_x}, {end_y}, {end_x}");
    debug!("{width},{height}");

    let mut y = start_y;
    while y < end_y {
        let mut x = start_x;
        while x < end_x {
            let chunk_min = MercatorPoint::new(x, y);
            let chunk_max =
                MercatorPoint::new(round(x + CHUNK_SIZE, 10), round(y + CHUNK_SIZE, 10));
            // debug!("Making chunk: {chunk_min:?}, {chunk_max:?}");
            let chunk = MercatorBox::new(chunk_min, chunk_max);
            chunks.push(chunk);
            x += CHUNK_SIZE;
            x = round(x, 4);
        }
        y += CHUNK_SIZE;
        y = round(y, 4);
        println!("{}", chunks.len());
    }

    Vec2D::new_from_vec(chunks, width, height).unwrap()
}

// #[test]
// fn split_geobox_test() {
//     logger::init(logger::Config::default());
//     let geo_box = GeoBox::new(
//         GeoPoint::new(40.720610, 73.925242),
//         GeoPoint::new(40.740610, 73.945242),
//     );
//     let chunks = split_geobox(&geo_box);

//     println!(
//         "{}, {}, {}, {}",
//         geo_box.min().lat(),
//         geo_box.min().lon(),
//         geo_box.max().lat(),
//         geo_box.max().lon()
//     );

//     for chunk in chunks.iter() {
//         println!(
//             "{}, {}, {}, {}",
//             chunk.min().lat(),
//             chunk.min().lon(),
//             chunk.max().lat(),
//             chunk.max().lon()
//         );
//     }

//     println!(
//         r#"({}, {}, {}, {}),"#,
//         round(geo_box.center().lat(), 5),
//         round(geo_box.center().lon(), 5),
//         round(geo_box.max().lat() - geo_box.min().lat(), 5),
//         round(geo_box.max().lon() - geo_box.min().lon(), 5),
//     );

//     for chunk in chunks.iter() {
//         println!(
//             r#"({}, {}, {}, {}),"#,
//             round(chunk.center().lat(), 5),
//             round(chunk.center().lon(), 5),
//             round(chunk.max().lat() - chunk.min().lat(), 5),
//             round(chunk.max().lon() - chunk.min().lon(), 5),
//         );
//     }
// }
#[test]
fn split_mercatorbox_test() {
    logger::init(logger::Config::default());
    let new_york = MercatorPoint::new(-8230433.491117454, 4972687.535733603);
    let mbox = MercatorBox::from_center_and_size(&new_york, (0.1, 0.1));
    let gbox = osm::coord::geo::GeoBox::new(
        osm::coord::convertion::mercator_to_geo(mbox.min()),
        osm::coord::convertion::mercator_to_geo(mbox.max()),
    );

    let mchunks = split_mercatorbox(&mbox);

    assert_eq!(
        mchunks,
        Vec2D::new_from_vec(
            vec![
                MercatorBox::new(
                    MercatorPoint::new(-8230500.0, 4972600.0),
                    MercatorPoint::new(-8230400.0, 4972700.0)
                ),
                MercatorBox::new(
                    MercatorPoint::new(-8230400.0, 4972600.0),
                    MercatorPoint::new(-8230300.0, 4972700.0)
                ),
                MercatorBox::new(
                    MercatorPoint::new(-8230500.0, 4972700.0),
                    MercatorPoint::new(-8230400.0, 4972800.0)
                ),
                MercatorBox::new(
                    MercatorPoint::new(-8230400.0, 4972700.0),
                    MercatorPoint::new(-8230300.0, 4972800.0)
                )
            ],
            2,
            2
        )
        .unwrap()
    );

    // println!(
    //     "{}, {}, {}, {}",
    //     gbox.min().lat(),
    //     gbox.min().lon(),
    //     gbox.max().lat(),
    //     gbox.max().lon()
    // );

    // for chunk in chunks.iter() {
    //     println!(
    //         "{}, {}, {}, {}",
    //         chunk.min().lat(),
    //         chunk.min().lon(),
    //         chunk.max().lat(),
    //         chunk.max().lon()
    //     );
    // }

    // println!(
    //     r#"({}, {}, {}, {}),"#,
    //     round(gbox.center().lat(), 5),
    //     round(gbox.center().lon(), 5),
    //     round(gbox.max().lat() - gbox.min().lat(), 5),
    //     round(gbox.max().lon() - gbox.min().lon(), 5),
    // );

    // for chunk in chunks.iter() {
    //     println!(
    //         r#"({}, {}, {}, {}),"#,
    //         round(chunk.center().lat(), 5),
    //         round(chunk.center().lon(), 5),
    //         round(chunk.max().lat() - chunk.min().lat(), 5),
    //         round(chunk.max().lon() - chunk.min().lon(), 5),
    //     );
    // }
}
