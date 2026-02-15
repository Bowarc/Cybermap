use dioxus::prelude::{
    Asset, Element, asset, component, dioxus_core, dioxus_elements, dioxus_signals, document,
    manganis, rsx,
};

const FOOTER_CSS: Asset = asset!("/assets/footer.css");

#[component]
pub fn Footer() -> Element {
    rsx! {
        document::Link{ rel:"stylesheet", href:FOOTER_CSS},

        footer {
            class:"cyber-razor-top bg-yellow fixed-bottom",
        }
    }
}
