use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/navbar.css");

#[component]
pub fn Navbar(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }
        div {
            id: "navbar",
            class: "cyber-razor-bottom bg-black",
            {children}
        }
    }
}
