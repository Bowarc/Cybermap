use osm::{
    coord::{
        convertion,
        geo::{GeoBox, GeoPoint},
        mercator::{MercatorBox, MercatorPoint},
    },
    element::{MercatorNWR, MercatorNode, MercatorWay, NWR},
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

const USER_AGENT: &str = "CybermapTesting/0.1.0 (linux; x86_64)";

const API_SERVERS: &[&str] = &[
    // "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
    "https://z.overpass-api.de/api/interpreter",
    "https://lz4.overpass-api.de/api/interpreter",
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
    // let cache: Arc<RwLock<Box<dyn Cache>>> =
    //     Arc::new(RwLock::new(Box::new(cache::CacheMap::new())));

    let api_pool = ServerPool::new(API_SERVERS, Duration::from_secs(1));

    let rate_limiter_config = warp_rate_limit::RateLimitConfig {
        max_requests: 4,
        window: Duration::from_secs(10),
        retry_after_format: warp_rate_limit::RetryAfterFormat::HttpDate,
        ip_header: "X-Forwarded-For".to_owned(),
    };

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

/// The idea is simple, the user sends a zone they wanna see, we fetch it for them.
/// In reality, it's a bit more complicated.
/// OSM is slow, especially when requested big areas,
/// My ideas is to use two things,
/// - Caching
/// - Fragmentation
///
/// Caching is easy, but two requests are virtually impossible be identical, even from the same user.
/// So I added a chunk system on top of that, and to 'standardize' chunks positions,
/// we round the coordinates to a given fraction (10th, in mercator coordinates).
///
/// Then we request the chunks one by one, cache the result and reconstruct the area from the data.
/// Bundle that up, send back to the clients and the next time, only the new chunks needs to be requested,
/// the rest can be reconstructed from previous request by using the cache.
///
/// We internally work in mercator coordinates, but OSM works in Geo coordinates,
/// WE do the translation and use geo ONLY when interacting with OSM.
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
    debug!("Split box: {chunk_rects:?}");
    let cache_read = cache.read().await;

    let mut chunks = HashMap::<(usize, usize), chunk::Chunk>::with_capacity(chunk_rects.len());

    debug!("Asking cache for known regions");
    // Fetch cached chunks
    {
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
                        "Something went wrong".to_string(),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }
            }
        }
        debug!("Found {} cached chunks", chunks.len());
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

    debug!(
        "Greedy chunk alg merged {} chunk to {}\n{to_be_requested:?}",
        chunk_rects.len(),
        to_be_requested.len()
    );

    // NOTE: This should probably hold mercator data directly instead of converting below
    let mut new_data: NWR = NWR::default();
    for (i, chunk_rect) in to_be_requested.iter().enumerate() {
        // NOTE: Would be nice to request a bit more than necessary in order to not miss any data (geo -> mercator error)
        // like 5% ?
        debug!("Requesting data for chunk n°{}. ({chunk_rect:?})", i + 1);
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
        let data = match request_area_with_url(gbox, api_url).await {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to request geobox {i}: {gbox:?} due to: {e}");
                continue;
            }
        };

        for (key, node) in data.nodes.into_iter() {
            if new_data.nodes.contains_key(&key) {
                warn!("New NWR already contains the node:\nKey: {key}\nData: {node:?}");
                continue;
            }
            new_data.nodes.insert(key, node);
        }

        for (key, way) in data.ways.into_iter() {
            // TODO, BUG.
            // Another call could have received an incomplete version of this OSMWay, this might be a ID duplicate
            // but it does not mean that the nodes or tags are the same
            // Find a way to merge them here
            if new_data.ways.contains_key(&key) {
                let new_ids = way.nodes.iter().map(|node| node.osm_id).collect::<Vec<_>>();
                let old_ids = new_data
                    .ways
                    .get(&key)
                    .unwrap()
                    .nodes
                    .iter()
                    .map(|node| node.osm_id)
                    .collect::<Vec<_>>();
                warn!(
                    "New NWR already contains the way:\nKey: {key}\nOld ids: {old_ids:?}\nNew ids: {new_ids:?}"
                );
                continue;
            }
            new_data.ways.insert(key, way);
        }

        // TODO: relations ?
    }

    // STATE: We have a lot of geo data, but we work in mercator coordinates
    debug!("Done, converting OSM data to mercator..");
    let new_nodes = new_data
        .nodes
        .into_iter()
        .map(|(id, node)| {
            (
                id,
                MercatorNode {
                    osm_id: id,
                    pos: convertion::geo_to_mercator(&node.pos),
                    tags: node.tags,
                },
            )
        })
        .collect::<HashMap<u64, MercatorNode>>();
    let new_ways = new_data
        .ways
        .into_iter()
        .map(|(id, way)| {
            let nodes = way
                .nodes
                .iter()
                // .flat_map(|osm_node| {
                //     // BUG: I'm expecting the node to be in the nodelist, but way nodes are stored only in ways
                //     // Idk where I store ways with only nodeids, but it's not here
                //     let res = new_nodes.get(&osm_node.osm_id);
                //     if res.is_none() {
                //         warn!(
                //             "A way node was not in the node pool\nId: {}\nNode: {osm_node:?}",
                //             osm_node.osm_id
                //         );
                //     }
                //     res
                // })
                .map(|osm_node| MercatorNode {
                    osm_id: osm_node.osm_id,
                    pos: convertion::geo_to_mercator(&osm_node.pos),
                    // FIXME: I don't like this clone, but nodes is an Arc, which i can't move out of
                    tags: osm_node.tags.clone(),
                })
                .collect::<Vec<MercatorNode>>();

            (
                id,
                MercatorWay {
                    osm_id: id,
                    nodes: nodes.into(),
                    tags: way.tags,
                },
            )
        })
        .collect::<HashMap<u64, MercatorWay>>();

    debug!(
        "Got {} new nodes: {new_nodes:?}\n and {} new ways: {new_ways:?}",
        new_nodes.len(),
        new_ways.len()
    );

    // This is OSM data, and should not be used as-is
    // drop(new_data);

    // STATE: We got the data, let's put it back into the corresponding chunks
    // NOTE: Now that I think of it, this and the last two steps could be merged into a single one
    // for .. in to_be_requested
    // request
    // convert to mercator
    // put the data in the right chunk
    // That would also remove the need to check every node received to find what chunk it should be assigned to
    // (Also, it is causing a BUG rn, and a requested way is not picked up by it's chunk and the chunk is staying empty)
    debug!("Putting back data into original chunks..");

    assert_eq!(to_request.len(), chunk_rects.len());
    let mut new_chunks: Vec<(chunk::Chunk, (usize, usize))> =
        Vec::with_capacity(to_be_requested.len()); // x, y
    for (x, y) in chunk_rects.iter() {
        if to_request.get(x, y) != Some(&true) {
            // that chunk already has data
            continue;
        }
        let chunk_rect = chunk_rects.get(x, y).unwrap();

        // todo!("Fill theses chunks with data using the data fetched by the requests above");
        let chunk_nodes = new_nodes
            .iter()
            .filter(|(_, node)| chunk_rect.contains_point(&node.pos))
            .map(|(id, node)| (*id, node.clone()))
            .collect::<HashMap<u64, MercatorNode>>();

        // Any way that contains at least one node in the chunk
        let chunk_ways = new_ways
            .iter()
            .filter(|(_, way)| {
                way.nodes
                    .iter()
                    .any(|node| chunk_rect.contains_point(&node.pos))
                // Maybe a better check would be to check the id against the chunk_nodes above
                // .any(|node| chunk_nodes.get(&node.osm_id).is_some())
            })
            .map(|(id, way)| (*id, way.clone()))
            .collect::<HashMap<u64, MercatorWay>>();

        // let chunk_truncated_ways = chunk_ways
        //     .iter()
        //     .map(|(id, way)| {
        //         // Something to consider also, is ways that go out of the area, and back in
        //         // Are they two ways now ?
        //         // And if so, how do we handle two ways with the same id, but a different nodeset
        //         //
        //         // Currently, we just filter out every node that is not in the area
        //         // This can create ways that are completely split, but this is a problem for another me
        //         // also, TODO, BUG
        //         let mut way_nodes = way.nodes.to_vec(); // This is a clone btw >:(
        //         way_nodes.retain(|node| chunk_rect.contains_point(&node.pos));

        //         (
        //             *id,
        //             way_nodes
        //                 .iter()
        //                 .map(|node| node.osm_id)
        //                 .collect::<Vec<u64>>(),
        //         )
        //     })
        //     .filter(|(_, way_nodesid)| !way_nodesid.is_empty())
        //     .collect::<HashMap<u64, Vec<u64>>>();

        let chunk = chunk::Chunk {
            zone: *chunk_rect,
            nodes: chunk_nodes,
            ways: chunk_ways,
            // non_truncated_ways: chunk_truncated_ways,
        };
        debug!("Chunk: {chunk:?}");
        new_chunks.push((chunk, (x, y)));
    }

    debug!("Writing to disk");
    // Write the new chunks to disk

    // Save to cache
    let mut cache_write = cache.write().await;

    for (chunk, pos) in new_chunks.into_iter() {
        // NOTE: Maybe don't write to disk empty chunks
        // TODO: Add a way to not write to disk failed chunks (where the request failed)

        if chunk.is_empty() {
            debug!("Skipping empty chunk: {pos:?}");
            continue;
        } else {
            debug!("Writing: {pos:?}");
        }

        let key = serde_json::to_string(&chunk.zone).unwrap();
        let data = serde_json::to_string(&chunk).unwrap();
        cache_write.insert(&key, &data).unwrap();

        // Push to chunks
        // BUG: Why is this here ?
        // Ig because we don't want to write non-new chunks ?
        // There has to be a better solution
        if let Some(replaced) = chunks.insert(pos, chunk) {
            warn!(
                "A new chunk somehow replaced one that was already saved. (Pos: {pos:?}, old chunk: {replaced:?})"
            );
        }
    }
    drop(cache_write);

    // todo!("Group the chunk grid (now full unless errors) into a big NWR");

    let mercator_nwr = merge_mercator_chunks(chunks);

    // todo!("Send back over the wire");
    let mercator_nwr_json = serde_json::to_string(&mercator_nwr).unwrap();

    Ok(warp::reply::with_status(mercator_nwr_json, StatusCode::OK))
}

fn merge_mercator_chunks(chunks: HashMap<(usize, usize), chunk::Chunk>) -> MercatorNWR {
    let mut mnwr = MercatorNWR::empty();

    for (_pos, chunk) in chunks {
        for (id, node) in chunk.nodes.into_iter() {
            if mnwr.nodes.contains_key(&id) {
                // NOTE: Maybe check and merge tags ?
                // Also make sure that they have the same position.
                continue;
            }
            mnwr.nodes.insert(id, node);
        }
        for (id, way) in chunk.ways.into_iter() {
            if mnwr.ways.contains_key(&id) {
                // NOTE: Need to merge, probably a method in the MercatorWay / MercatorNode structs
                // trying to merge self with another one.
                // If they are contiguous, perfect, easy merge, add tags, done
                // but if not, i'm not sure how to proceed.
                // Maybe check if one's a superset (⊃) of the other ?
                // If not, maybe one is supposed to be inside the other one ?
                // A = 123489, B = 34567 OR B = 345678
                // Even though we can't actually see the numbers, we could find the start and end of B in A's nodes)
                // It would be a bit more difficult if they don't line up at all (B = 567),
                // then we can't know if B is before, inside or after A.
                continue;
            }
            mnwr.ways.insert(id, way);
        }
    }

    mnwr
}

async fn request_area_with_url(gbox: GeoBox, api_url: Arc<str>) -> Result<NWR, String> {
    debug!("Request area: {gbox:#?} at {api_url}");
    // TODO: Pass real client
    let client = reqwest::Client::new();

    let query = format!(
        r#"
[out:json][timeout:360][bbox:{},{},{},{}];
(
    way
    ['name']
    ['highway']
    ['highway' !~ 'path']
    ['highway' !~ 'steps']
    ['highway' !~ 'motorway']
    ['highway' !~ 'motorway_link']
    ['highway' !~ 'raceway']
    ['highway' !~ 'bridleway']
    ['highway' !~ 'proposed']
    ['highway' !~ 'construction']
    ['highway' !~ 'elevator']
    ['highway' !~ 'bus_guideway']
    ['highway' !~ 'footway']
    ['highway' !~ 'cycleway']
    ['foot' !~ 'no']
    ['access' !~ 'private']
    ['access' !~ 'no'];
);
(._;>;);
out;
(
    way["building"];
);
(._;>;);
out;"#,
        gbox.min().lat(),
        gbox.min().lon(),
        gbox.max().lat(),
        gbox.max().lon()
    );

    trace!("Sending: {query}");

    let request = client
        .get(api_url.to_string()) // My dev proxy is somehow turning them into 'bad requests'
        .query(&[("data", &query)])
        .header("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| {
            error!("An error occured while building the request with user data: {e}");
            OSMProxyRejection::InvalidUserData
        })
        .unwrap();

    let response = client
        .execute(request)
        .await
        .map_err(|e| {
            error!(
                "An error occured while querying api server ({}) with data:\n{}\n{e}",
                api_url, query
            );
            OSMProxyRejection::InvalidUserData
        })
        .unwrap();

    if !response.status().is_success() {
        let status_code = response.status();

        error!(
            "Got a non-success code when querying api server ({}): {}",
            api_url,
            response.status()
        );

        if status_code.is_client_error() {
            return Err(String::from("Client error"));
        }

        return Err(String::from("Server failure"));
    }

    let res_body = response.json().await.map_err(|e| {
        error!(
            "An error occured while unpacking the reponse of the api server ({}): {e}",
            api_url
        );
        String::from("Failed to unpack server response")
    })?;

    osm::parsing::parse_osm_json(res_body).map_err(|e| ToString::to_string(&e))
}

fn greedy_chunk(
    chunk_rects: &Vec2D<MercatorBox>,
    mut chunks_to_request: Vec2D<bool>,
) -> Vec<MercatorBox> {
    assert_ne!(chunk_rects.len(), 0, "No chunks to merge back together");
    assert_eq!(
        chunk_rects.width(),
        chunks_to_request.width(),
        "The two arrays needs to have the same shape"
    );
    assert_eq!(
        chunk_rects.height(),
        chunks_to_request.height(),
        "The two arrays needs to have the same shape"
    );

    // let xy_to_index = |x: u16, y: u16| -> usize { (y * width as u16 + x) as usize };

    let mut boxes_to_request = Vec::<MercatorBox>::new();

    debug!("Greedy chunk start, got {} chunks", chunk_rects.len());

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

        debug!("Start: {start:#?}\nEnd: {end:#?}");

        let new_box = MercatorBox::new(
            MercatorPoint::new(start.min().x(), end.min().y()),
            MercatorPoint::new(end.max().x(), start.max().y()),
        );
        debug!("New box: {new_box:#?}");

        boxes_to_request.push(new_box);
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

        // let chunk_full_ways = chunk_ways
        //     .keys()
        //     .map(|id| ways.get(id).unwrap()) // this unwrap should NEVER fail as chunk_ways is a subset of ways
        //     .map(|way| {
        //         (
        //             way.osm_id,
        //             way.nodes
        //                 .iter()
        //                 .map(|node| node.osm_id)
        //                 .collect::<Vec<u64>>(),
        //         )
        //     })
        //     .collect::<HashMap<u64, Vec<u64>>>();

        let chunk = chunk::Chunk {
            zone: *chunk_rect,
            nodes: chunk_nodes,
            ways: chunk_ways,
            // non_truncated_ways: chunk_full_ways,
        };

        chunks.push(chunk);
    }

    Vec2D::new_from_vec(chunks, chunk_rects.width(), chunk_rects.height()).unwrap()
}

/// Split an area into a list of chunks with standardized centers (to the 10th mercator decimal)
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

    debug!("Splitting mercatorbox: {mbox:?}");
    debug!(
        "starty: {start_y}, startx: {start_x}\nendy: {end_y}, endx: {end_x}\nWidth: {width}, height: {height}"
    );

    let mut y = end_y - CHUNK_SIZE;
    while y >= start_y {
        let mut x = start_x;
        while x < end_x {
            let chunk_min = MercatorPoint::new(x, y);
            let chunk_max =
                MercatorPoint::new(round(x + CHUNK_SIZE, 10), round(y + CHUNK_SIZE, 10));
            // debug!("Creating chunk: {chunk_min:?}, {chunk_max:?}");
            let chunk = MercatorBox::new(chunk_min, chunk_max);
            chunks.push(chunk);
            x = round(x + CHUNK_SIZE, 4);
        }
        y = round(y - CHUNK_SIZE, 4);
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

#[test]
fn split_mercatorbox_visualisation() {
    let bx = GeoBox::from_center_and_size(GeoPoint::new(10., 10.), (0.5, 0.5));

    let mbx = bx.to_mercator();

    let chunks = split_mercatorbox(&mbx);

    let display = |geobox: &GeoBox| {
        println!(
            "({}, {}, {}, {}),",
            round(geobox.center().lat(), 5),
            round(geobox.center().lon(), 5),
            round(geobox.max().lat() - geobox.min().lat(), 5),
            round(geobox.max().lon() - geobox.min().lon(), 5),
        );
    };

    let display_mercator = |mercatorbox: &MercatorBox| {
        println!(
            "({}, {}, {}, {}),",
            round(mercatorbox.max().x() - mercatorbox.width() / 2., 5),
            round(mercatorbox.max().y() - mercatorbox.height() / 2., 5),
            round(mercatorbox.width(), 5),
            round(mercatorbox.height(), 5),
        );
    };

    // First print is main area
    display_mercator(&mbx);

    for pos in chunks.iter() {
        let chunk = chunks.get(pos.0, pos.1).unwrap(); // Cannot fail

        display_mercator(&chunk);
    }
}
