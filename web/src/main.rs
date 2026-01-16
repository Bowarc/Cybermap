use dioxus::prelude::*;

use ui::Navbar;
use views::{Home, Map};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[route("/")]
    Home {},
    #[route("/map")]
    Map {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const CYBERPUNK_CSS: Asset = asset!("/assets/cyberpunk.css");
const _: Asset = asset!("/assets/fonts/BlenderProBook.woff2");
const _: Asset = asset!("/assets/fonts/Cyberpunk.otf");
const _: Asset = asset!("/assets/fonts/Oxanium.woff2");

fn main() {
    dioxus::logger::init(dioxus::logger::tracing::Level::DEBUG).expect("failed to init logger");
    debug!("Init");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: CYBERPUNK_CSS }

        Router::<Route> {}
    }
}

#[component]
fn WebNavbar() -> Element {
    rsx! {
        Navbar {
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::Map {},
                "Map"
            }
        }

        Outlet::<Route> {}
    }
}
