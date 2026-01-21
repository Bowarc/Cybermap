pub mod api;

mod hero;
pub use hero::Hero;

mod navbar;
pub use navbar::Navbar;

mod svgmap;
pub use svgmap::SvgMap;

mod footer;
pub use footer::Footer;

mod svg_element;

mod map;
pub use map::Map;

const MAIN_CSS: Asset = asset!("/assets/main.css");
pub const CYBERPUNK_CSS: Asset = asset!("/assets/cyberpunk.css",);

use dioxus::{
    core::Element,
    document::Link,
    prelude::{Asset, AssetOptions, asset, component, dioxus_core, dioxus_signals, manganis, rsx},
};

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

#[component]
pub fn UiResources() -> Element {
    rsx! {
        Link { rel: "stylesheet", href: MAIN_CSS }
        Link { rel: "stylesheet", href: CYBERPUNK_CSS }
    }
}
