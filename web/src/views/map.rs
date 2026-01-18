use dioxus::{
    core::{Callback, Element, provide_context, use_hook},
    document,
    hooks::{to_owned, use_signal},
    html::geometry::PixelsSize,
    prelude::{
        Asset, asset, component, debug, dioxus_core, dioxus_elements, dioxus_signals, error,
        manganis, rsx,
    },
    signals::{Signal, WritableExt as _},
};
use dioxus_sdk_geolocation::{Geolocator, PowerMode};
use dioxus_sdk_time::sleep;
use osm::{
    coord::geo::{GeoBox, GeoPoint},
    element::NWR,
};
use reqwest::Client;
use std::{rc::Rc, time::Duration};
use ui::SvgMap;

const API_URL: &str = "http://127.0.0.1:42061/overpass_api";
const MAP_CSS: Asset = asset!("/assets/map.css");

enum QueryError {
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
}

async fn query(geobox: GeoBox, url: &str) -> Result<NWR, QueryError> {
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
        out;
        "#,
        geobox.min().lat(),
        geobox.min().lon(),
        geobox.max().lat(),
        geobox.max().lon()
    );

    let client = Client::new();

    debug!("Query: {query:?}");

    let json_value = {
        let request = client
            .get(url)
            .timeout(Duration::from_secs_f32(10.))
            .header("cybermap", "8b3d00bf-b0cc-4a7d-b389-9c0e9d0688f8")
            .query(&[("data", query)]);

        debug!("Sending: {request:?}");

        let res = request.send().await.map_err(QueryError::Reqwest)?;

        res.json::<serde_json::Value>()
            .await
            .map_err(QueryError::Reqwest)?
    };

    osm::parsing::parse_osm_json(json_value).map_err(QueryError::SerdeJson)
}

#[component]
pub fn Map() -> Element {
    let screen_size = use_signal(|| None as Option<PixelsSize>);

    let geolocator = use_hook(|| {
        let geolocator = Signal::new(std::sync::Arc::new(
            Geolocator::new(PowerMode::High).unwrap(),
        ));
        provide_context(geolocator)
    });

    let mut osm_data = use_signal(|| None as Option<(GeoBox, Rc<NWR>)>);

    let range_km = use_signal(|| 1.);

    let mut update_data = async move || {
        let Some(screen_size) = screen_size() else {
            error!("Called map update data with None screen size");
            return;
        };
        let current_geocords = match geolocator().get_coordinates().await {
            Ok(coords) => coords,
            Err(e) => {
                error!("Failed to retrieve current geocordinates due to: {e}");
                return;
            }
        };

        debug!("Got coordinates: {current_geocords:?}");

        let box_center = GeoPoint::new(current_geocords.latitude, current_geocords.longitude);
        let range_km = range_km();
        let scale_factor = range_km / screen_size.width.max(screen_size.height);
        let box_size = (
            screen_size.width * scale_factor,
            screen_size.height * scale_factor,
        );

        // TODO: Redo that to make it dynamic
        let mut retries = 0;
        const MAX_RETRIES: u8 = 2;
        while retries < MAX_RETRIES {
            let geobox = GeoBox::from_center_and_size(box_center, box_size);
            let nwr = match query(geobox, API_URL).await {
                Ok(nwr) => nwr,
                Err(QueryError::Reqwest(e)) => {
                    error!("Overpass api request failled due to: {e}");
                    retries += 1;
                    debug!("Retrying");
                    continue;
                }
                Err(QueryError::SerdeJson(e)) => {
                    error!("Failed to decode Overpass api response due to: {e}");
                    return;
                }
            };
            debug!("Got NWR in {} tries", retries + 1);
            osm_data.set(Some((geobox, Rc::new(nwr))));
            break;
        }
    };

    let on_resize = Callback::<PixelsSize, ()>::new(move |svg_size: PixelsSize| {
        to_owned![screen_size];
        async move {
            screen_size.set(Some(svg_size));
            debug!("Callback: {svg_size:?}");
            debug!("Set screen size: {:?}", screen_size());

            sleep(Duration::from_secs_f32(0.5)).await;

            if screen_size() != Some(svg_size) {
                // Another resize event has been called, this is no longer up to date
                return;
            }
            update_data().await;
        }
    });

    let on_km_range_change = Callback::<f64, ()>::new(move |km_range: f64| {
        to_owned![range_km];
        async move {
            range_km.set(km_range);
            sleep(Duration::from_secs_f32(0.5)).await;
            if range_km() != km_range {
                // Another range update event has been called, this is no longer up to date
                return;
            }
            update_data().await;
        }
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS }

        div {
            id: "map-root",

            input {
                id: "km_range_input",
                type: "range",
                min: 0.5,
                max: 3.,
                step: 0.1,
                value: range_km(),

                oninput: move |cx| async move {
                    on_km_range_change(cx.data.value().parse::<f64>().unwrap())
                }
            }

            SvgMap {
                osm_data: osm_data(),
                onresize: on_resize
            },
        }
    }
}
