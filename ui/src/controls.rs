#![allow(unused_imports)]

use dioxus::{
    core::{Element, provide_context, use_hook},
    document,
    hooks::use_signal,
    html::geometry::PixelsSize,
    prelude::{
        Asset, Props, asset, component, debug, dioxus_core, dioxus_elements, dioxus_signals, error,
        manganis, rsx,
    },
    signals::{Signal, WritableExt as _},
};

const CONTROLS_CSS: Asset = asset!("/assets/controls.css");

pub const RANGE_KM_SLIDER_RANGE: std::ops::Range<f64> = (0.1)..3.;

#[component]
pub fn Controls(
    controls_open: Signal<bool>,
    osm_data_signal_bundle: crate::map::OsmSignalBundle,
) -> Element {
    if !controls_open() {
        rsx! {
            link { rel:"stylesheet", href: CONTROLS_CSS },

            button {
                class: "cyber-button-small bg-yellow fg-black",
                style: "font-size: 0.7em",
                onclick: move |_event|{
                    controls_open.set(true)
                },

                "Controls"

                span {
                    class: "glitchtext",
                    "H4CK3D"
                }
            }

        }
    } else {
        let screen_size_str = osm_data_signal_bundle
            .screen_size()
            .map(|size| format!("{size:.1?}"))
            .unwrap_or_else(|| String::from("None"));

        rsx! {
            link { rel:"stylesheet", href: CONTROLS_CSS },

            div {
                style: "width: fit-content; font-size:0.8em",
                class: "cyber-razor-bottom bg-yellow fg-black",
                "Svg map size: {screen_size_str}",
                br {}

                "Zoom: "
                br{}
                input {
                    id: "km_range_input",
                    type: "range",
                    min: RANGE_KM_SLIDER_RANGE.start,
                    max: RANGE_KM_SLIDER_RANGE.end,
                    step: 0.1,
                    value: osm_data_signal_bundle.range_km(),

                    oninput: move |cx| async move {
                        osm_data_signal_bundle.set_range(cx.data.value().parse::<f64>().unwrap()).await;
                    }
                }
            }
        }
    }
}
