use dioxus::{html::geometry::PixelsSize, prelude::*};
use dioxus_elements::geometry::WheelDelta;

use crate::svg_element::SVGElement;

const MAP_CSS: Asset = asset!("/assets/svgmap.css");
const SVG_ID: &str = "map-svg";

#[component]
pub fn SvgMap(controls_open: Signal<bool>, osm_data_signal_bundle: crate::map::OsmSignalBundle) -> Element {
    let mut shapes = use_signal(Vec::<SVGElement>::new);

    let mut svg_dimensions = use_signal(|| None as Option<PixelsSize>);

    let mut mouse_pos = use_signal(|| (0., 0.));

    if let Some(size) = svg_dimensions()
        && let Some((geobox, nwr)) = osm_data_signal_bundle.osm_data()
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

        svg {
            id: SVG_ID,
            width: "100%",
            height: "100%",


            onmousedown: move |_ev|{
                controls_open.set(false);
            },

            // Web
            onmousemove: move |event| {
                // debug!("Mouse move");
                let coords = event.data.client_coordinates();
                mouse_pos.set((coords.x, coords.y));
            },
            onwheel: move |event: Event<WheelData>| async move {
                let WheelDelta::Lines(wheel_delta_vector) = event.data.delta() else{
                    error!("Scroll data not of the 'Lines' variant: {:?}", event.data.delta());
                    return;
                };
                let v = wheel_delta_vector.y;
                osm_data_signal_bundle.set_range(osm_data_signal_bundle.range_km() + v * 0.01).await;
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
                osm_data_signal_bundle.set_screen_size(size).await;
            }},


            for shape in shapes.iter(){{
                shape.to_element()
            }}
        }
    }
}
