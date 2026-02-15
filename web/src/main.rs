use dioxus::prelude::*;
use ui::{Footer, Map, Navbar, UiResources};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[route("/")]
    Map {},
}

const FAVICON: Asset = asset!(
    "/assets/favicon.ico",
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

        UiResources {  }
        Router::<Route> {}
        Footer { }
    }
}

#[component]
fn WebNavbar() -> Element {
    rsx! {
        Navbar {
            Link {
                to: Route::Map {},
                "Map"
            }
        }

        Outlet::<Route> {}
    }
}
