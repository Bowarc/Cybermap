use dioxus::prelude::*;

const NAVBAR_CSS: Asset = asset!("/assets/navbar.css");
const HOME_CSS: Asset = asset!("/assets/home.css");

#[component]
pub fn Navbar(children: Element) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: NAVBAR_CSS }
        div {
            id: "navbar",
            class: "cyber-razor-bottom bg-yellow fg-black",
            {children}

            document::Link{ rel: "stylesheet", href: HOME_CSS }

            div {
                class: "cyberpunk-font-og cybermap-title fg-black",
                style: "margin: auto;",
                span { class: "fg-black c", "C" }
                "yber"
                span { class: "fg-black m", "M" }
                "ap"
            }
        }
    }
}
