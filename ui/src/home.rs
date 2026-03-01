use dioxus::{
    document::Link,
    prelude::{
        Asset, Element, asset, component, dioxus_core, dioxus_elements, dioxus_signals, manganis,
        rsx,
    },
};

const HOME_CSS: Asset = asset!("/assets/home.css");

#[component]
pub fn Home() -> Element {
    rsx! {
        Link{ rel: "stylesheet", href: HOME_CSS }

        div {
            class: "cyberpunk-font-og cybermap-title fg-black",
            span { class: "fg-black c", "C" }
            "yber"
            span { class: "fg-black m", "M" }
            "ap"
        }
    }
}
