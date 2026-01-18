use dioxus::prelude::*;

use ui::{Footer, Navbar};
use views::{Home, Map};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(MobileNavbar)]
    #[route("/")]
    Home {},
    #[route("/map")]
    Map {},
}

const MAIN_CSS: Asset = asset!("/assets/main.css");
const CYBERPUNK_CSS: Asset = asset!("/assets/cyberpunk.css");

const _: Asset = asset!("/assets/BlenderProBook.woff2", AssetOptions::builder().with_hash_suffix(false));
const _: Asset = asset!("/assets/Cyberpunk.otf", AssetOptions::builder().with_hash_suffix(false));
const _: Asset = asset!("/assets/Oxanium.woff2", AssetOptions::builder().with_hash_suffix(false));


fn main() {
    dioxus::logger::init(dioxus::logger::tracing::Level::DEBUG).expect("failed to init logger");
    debug!("Init");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: CYBERPUNK_CSS }

        Router::<Route> {}
        Footer { }
    }
}

/// A mobile-specific Router around the shared `Navbar` component
/// which allows us to use the mobile-specific `Route` enum.
#[component]
fn MobileNavbar() -> Element {
    // let wv = &dioxus::mobile::window().webview;
    // let w = dioxus::mobile::window().window.clone();
    // let ips = w.inner_size();
    // let ops = w.outer_size();
    // let sf = w.scale_factor();

    // let ils = ips.to_logical::<u32>(sf);
    // let ols = ops.to_logical::<u32>(sf);

    // debug!("Inner phys size: {ips:?}");
    // debug!("Outer phys size: {ops:?}");
    // debug!("Scale factor: {sf}");

    // debug!("Inner log size: {ils:?}");
    // debug!("Outer log size: {ols:?}");

    // let measured_size = (412., 890.);

    // let measured_sf = (ils.width as f64 /measured_size.0, ils.height as f64 / measured_size.1);

    // debug!("Measured sf: {measured_sf:?}");

    // let bounds = wv.bounds();

    // debug!("Webview bounds: {bounds:?}");

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
