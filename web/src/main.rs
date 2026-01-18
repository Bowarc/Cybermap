use dioxus::prelude::*;

use ui::{Footer, Navbar};
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

const FAVICON: Asset = asset!(
    "/assets/favicon.ico",
    AssetOptions::builder().with_hash_suffix(false)
);
const MAIN_CSS: Asset = asset!("/assets/main.css",);
const CYBERPUNK_CSS: Asset = asset!("/assets/cyberpunk.css",);

// NOTE for later:
// If despite being here but not used in rust, the asset is not imported correctly
// use the #[used]
// For more info, check .with_hash_suffix's note
const _: Asset = asset!(
    "/assets/BlenderProBook.woff2",
    AssetOptions::builder().with_hash_suffix(false)
);
const _: Asset = asset!(
    "/assets/Cyberpunk.otf",
    AssetOptions::builder().with_hash_suffix(false)
);
const _: Asset = asset!(
    "/assets/Oxanium.woff2",
    AssetOptions::builder().with_hash_suffix(false)
);

fn main() {
    dioxus::logger::init(dioxus::logger::tracing::Level::DEBUG).expect("failed to init logger");
    debug!("Init");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: CYBERPUNK_CSS }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
        Footer { }
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
