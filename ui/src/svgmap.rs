use std::rc::Rc;

use dioxus::{html::geometry::PixelsSize, prelude::*};
use osm::{
    coord::{
        geo::{GeoBox, GeoPoint},
        screen::ScreenPoint,
    },
    element::NWR,
};

use crate::svg_element::SVGElement;

const MAP_CSS: Asset = asset!("/assets/styling/map.css");
const SVG_ID: &str = "map-svg";

#[component]
pub fn SvgMap(osm_data: Option<(GeoBox, Rc<NWR>)>, onresize: Callback<PixelsSize, ()>) -> Element {
    let mut shapes = use_signal(Vec::<SVGElement>::new);

    let mut svg_dimensions = use_signal(|| None as Option<PixelsSize>);

    let mut mouse_pos = use_signal(|| (0., 0.));

    if let Some(size) = svg_dimensions()
        && let Some((geobox, nwr)) = osm_data
    {
        debug!(
            "OSMData size: {} for a box of {}x{} km",
            nwr.total_count(),
            geobox.width_km(),
            geobox.height_km()
        );
        shapes.set(crate::svg_element::generate_from_osm(size, &geobox, &nwr));
    }

    debug!("Map svg render");

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS }

        div {
            "Svg map size: {svg_dimensions():.1?}"
            br {}
            "Mouse: {mouse_pos:.1?}"
        }

        svg {
            id: SVG_ID,
            width: "100%",
            height: "100%",

            // Web
            onmousemove: move |event| {
                // debug!("Mouse move");
                let coords = event.data.client_coordinates();
                mouse_pos.set((coords.x, coords.y));
            },

            // Mobile
            ontouchmove: move |event| {
                let coords = event.touches().first().unwrap().client_coordinates();
                mouse_pos.set((coords.x, coords.y));
            },

            onresize: move |cx| async move { 'onresize: {
                let size = match cx.data().get_content_box_size() {
                    Ok(size) => size,
                    Err(e) => {
                        error!("Failed to unpack map's svg onresize event due to: {e}");
                        break 'onresize
                    }
                };
                debug!("SVGMAP RESIZED: {}x{}", size.width, size.height);
                svg_dimensions.set(Some(size));
                onresize(size);
            }},

            for shape in shapes.iter(){{
                shape.to_element()
            }}
        }
    }
}
