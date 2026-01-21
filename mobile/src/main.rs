use dioxus::prelude::*;

use ui::{Footer, Navbar, Map, UiResources};
use views::{Home};

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

fn main() {
    dioxus::logger::init(dioxus::logger::tracing::Level::DEBUG).expect("failed to init logger");
    debug!("Init");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {

    rsx! {
        UiResources {  }
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
