use crate::api;
use crate::{Controls, SvgMap, controls::RANGE_KM_SLIDER_RANGE};
use dioxus::{
    core::{Element, provide_context, use_hook},
    document,
    hooks::use_signal,
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
use std::{rc::Rc, sync::Arc, time::Duration};

const API_URL: &str = "http://127.0.0.1:42061/overpass_api";
const MAP_CSS: Asset = asset!("/assets/map.css");

#[derive(Clone, Copy, PartialEq)]
pub struct OsmSignalBundle {
    osm_data: Signal<Option<(GeoBox, Rc<NWR>)>>,
    range_km: Signal<f64>,
    screen_size: Signal<Option<PixelsSize>>,
    geolocator: Signal<Arc<Geolocator>>,
}

impl OsmSignalBundle {
    pub fn osm_data(&self) -> Option<(GeoBox, Rc<NWR>)> {
        (self.osm_data)()
    }
    pub fn range_km(&self) -> f64 {
        (self.range_km)()
    }
    pub async fn set_range(&mut self, new_range: f64) {
        if new_range == (self.range_km)() || !RANGE_KM_SLIDER_RANGE.contains(&new_range) {
            return;
        }

        self.range_km.set(new_range);
        self.update().await
    }

    pub fn screen_size(&self) -> Option<PixelsSize> {
        (self.screen_size)()
    }

    pub async fn set_screen_size(&mut self, new_screen_size: PixelsSize) {
        if Some(new_screen_size) == (self.screen_size)() {
            return;
        }
        self.screen_size.set(Some(new_screen_size));
        self.update().await
    }

    pub async fn update(&mut self) {
        // custom 'debounce' system

        let Some(screen_size) = (self.screen_size)() else {
            error!("Called map update data with None screen size");
            return;
        };
        let current_geocords = match (self.geolocator)().get_coordinates().await {
            Ok(coords) => coords,
            Err(e) => {
                error!("Failed to retrieve current geocordinates due to: {e}");
                return;
            }
        };

        debug!("Got coordinates: {current_geocords:?}");

        let box_center = GeoPoint::new(current_geocords.latitude, current_geocords.longitude);
        let box_size = {
            let range_km = (self.range_km)();
            let scale_factor = range_km / screen_size.width.max(screen_size.height);
            (
                screen_size.width * scale_factor,
                screen_size.height * scale_factor,
            )
        };
        let geobox = GeoBox::from_center_and_size(box_center, box_size);

        if let Some((_old_box, old_data)) = self.osm_data() {
            self.osm_data.set(Some((geobox, old_data)));
        }

        {
            let dependencies = || ((self.screen_size)(), (self.range_km)());

            // Everything that his function depends on
            let init_data = dependencies();
            // Wait for maybe changes
            sleep(Duration::from_secs_f32(0.5)).await;
            if dependencies() != init_data {
                debug!("Debounce test failed, stopping");
                return;
            }
        }
        debug!("Debounce test succeded, continuing");

        if let Some(nwr) = api::query_with_retries(geobox, API_URL, 1).await {
            (self.osm_data).set(Some((geobox, Rc::new(nwr))));
        }
    }
}

#[component]
pub fn Map() -> Element {
    let osm_data = use_signal(|| None as Option<(GeoBox, Rc<NWR>)>);
    let range_km = use_signal(|| 1.);
    let screen_size = use_signal(|| None as Option<PixelsSize>);
    let geolocator = use_hook(|| {
        let geolocator = Signal::new(Arc::new(Geolocator::new(PowerMode::High).unwrap()));
        provide_context(geolocator)
    });

    let controls_open = use_signal(|| false);

    let osm_data_signal_bundle = use_hook(|| OsmSignalBundle {
        osm_data,
        range_km,
        screen_size,
        geolocator,
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS },


        div {
            id: "map-root",

            Controls {
                controls_open,
                osm_data_signal_bundle
            }

            SvgMap {
                controls_open,
                osm_data_signal_bundle
            },
        },
    }
}
