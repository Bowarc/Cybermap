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
use std::{rc::Rc, time::Duration};
use crate::SvgMap;
use crate::api;

const API_URL: &str = "http://127.0.0.1:42061/overpass_api";
const MAP_CSS: Asset = asset!("/assets/map.css");

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

    let mut update_data_debounced = async move || {
        // custom 'debounce' system
        {
            let dependencies = || (screen_size(), range_km());

            // Everything that his function depends on
            let init_data = dependencies();
            // Wait for maybe changes
            sleep(Duration::from_secs_f32(0.5)).await;
            if dependencies() != init_data {
                error!("Debounce test failed, stopping");
                return;
            }
        }
        error!("Debounce test succeded, continuing");

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
        let box_size = {
            let range_km = range_km();
            let scale_factor = range_km / screen_size.width.max(screen_size.height);
            (
                screen_size.width * scale_factor,
                screen_size.height * scale_factor,
            )
        };
        let geobox = GeoBox::from_center_and_size(box_center, box_size);

        if let Some(nwr) = api::query_with_retries(geobox, API_URL, 2).await {
            osm_data.set(Some((geobox, Rc::new(nwr))));
        }
    };

    let on_resize = Callback::<PixelsSize, ()>::new(move |svg_size: PixelsSize| {
        to_owned![screen_size];
        async move {
            screen_size.set(Some(svg_size));
            debug!("Callback: {svg_size:?}");
            debug!("Set screen size: {:?}", screen_size());

            update_data_debounced().await;
        }
    });

    let on_km_range_change = Callback::<f64, ()>::new(move |km_range: f64| {
        to_owned![range_km];
        async move {
            range_km.set(km_range);

            update_data_debounced().await;
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
