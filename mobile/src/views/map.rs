use dioxus::{html::geometry::PixelsSize, prelude::*};
use osm::{
    coord::geo::{GeoBox, GeoPoint},
    element::NWR,
};
use reqwest::Client;
use std::{rc::Rc, time::Duration};
use ui::SvgMap;

const API_URL: &str = "http://10.0.2.2:42061/overpass_api";

// enum QueryError {
//     Reqwest(reqwest::Error),
//     SerdeJson(serde_json::Error),
// }

async fn query(geobox: GeoBox, url: &str) -> std::rc::Rc<[NWR]> {
    let query = format!(
        r#"
            [out:json][timeout:360][bbox:{},{},{},{}];
            (way
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
        "#,
        geobox.min().lat(),
        geobox.min().lon(),
        geobox.max().lat(),
        geobox.max().lon()
    );

    let client = Client::new();

    debug!("Query: {query:?}");

    let json_value = 'req: {
        let request = client
            .get(url)
            .timeout(Duration::from_secs_f32(10.))
            .header("cybermap", "8b3d00bf-b0cc-4a7d-b389-9c0e9d0688f8")
            .query(&[("data", query)]);

        debug!("Sending: {request:?}");

        let res = match request.send().await {
            Ok(res) => res,
            Err(e) => {
                error!("Failed to send the request due to: {e:?}");
                break 'req serde_json::Value::from("");
            }
        };

        match res.json::<serde_json::Value>().await {
            Ok(v) => v,
            Err(e) => {
                error!("Faield to get the json body of the response due to: {e:?}");
                serde_json::Value::from("")
            }
        }
    };

    osm::parsing::parse_osm_json(json_value)
        .map_err(|e| error!("Failed to parse osm json data due to: {e}"))
        .unwrap_or_default()
}

#[component]
pub fn Map() -> Element {
    let screen_size = use_signal(|| None as Option<PixelsSize>);

    let mut osm_data = use_signal(|| None as Option<(GeoBox, Rc<[NWR]>)>);

    let on_resize = Callback::<PixelsSize, ()>::new(move |svg_size: PixelsSize| {
        to_owned![screen_size];
        async move {
            screen_size.set(Some(svg_size));
            debug!("Callback: {svg_size:?}");
            debug!("Set screen size: {:?}", screen_size());

            let box_center = todo!();
            let range_km = 1.;
            let scale_factor = range_km / svg_size.width.max(svg_size.height);
            let box_size = (
                svg_size.width * scale_factor,
                svg_size.height * scale_factor,
            );

            // TODO: Redo that to make it dynamic
            let geobox = GeoBox::from_center_and_size(box_center, box_size);
            let nwr = query(geobox, API_URL).await;
            osm_data.set(Some((geobox, nwr)));
        }
    });

    rsx! {
        div {

            // onresize: move |cx| async move { 'onresize: {
            //     let size = match cx.data().get_content_box_size() {
            //         Ok(size) => size,
            //         Err(e) => {
            //             error!("Failed to unpack wrapper onresize event due to: {e}");
            //             break 'onresize
            //         }
            //     };
            //     debug!("Wrapper div RESIZED: {}x{}", size.width, size.height);
            //     screen_size.set(Some(size));
            //     if osm_data().is_none() && size.width != 0. && size.height != 0.{
            //         on_resize(size);
            //     }
            // }},

            SvgMap {
                osm_data: osm_data(),
                onresize: on_resize

            },
        }
    }
}
