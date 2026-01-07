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

fn generate_shapes(svg_size: PixelsSize, bx: &GeoBox, nwr: &[NWR]) -> Vec<SVGElement> {
    let map_pt = |pt: GeoPoint| -> ScreenPoint {
        ScreenPoint {
            // x: svg_size.width * ((pt.lon() - bx.min().lon()) / bx.width()),
            // y: svg_size.height * (1. - (pt.lat() - bx.min().lat()) / bx.height()),
            x: svg_size.width * ((pt.lon() - bx.min().lon()) / (bx.max().lon() - bx.min().lon())),
            y: svg_size.height
                * (1. - (pt.lat() - bx.min().lat()) / (bx.max().lat() - bx.min().lat())),
        }
    };

    let scale_km = svg_size.width / bx.width_km(); // Used to scale real world sizes to the viewport
    debug!("Scale km: {scale_km}");

    let mut shapes = Vec::new();
    let mut roads = Vec::new();
    // let mut land = Vec::new();

    for node in nwr.iter() {
        match node {
            NWR::Node(node) => {
                if node.tags.get("highway").map(|s| &**s) == Some("crossing") {
                    crate::svg_element::gen_crosswalk(node, &mut shapes, map_pt, scale_km);
                } else if node.tags.get("highway").map(|s| &**s) == Some("traffic_signals") {
                    crate::svg_element::gen_traficlight(node, &mut shapes, map_pt, scale_km);
                };
            }
            NWR::Way(way) => {
                if way.nodes.is_empty() {
                    debug!("Skipping nodeless way: {way:?}");
                    continue;
                }
                if way.tags.contains_key("area") {
                    debug!(
                        "Skipping area: {}",
                        way.tags
                            .get("name")
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{way:?}"))
                    );
                    continue;
                }

                crate::svg_element::gen_way(way, &mut roads, map_pt, scale_km);
            }
            _ => (),
        }
    }

    roads.extend(shapes);
    roads
}

#[component]
pub fn SvgMap(
    osm_data: Option<(GeoBox, Rc<[NWR]>)>,
    onresize: Callback<PixelsSize, ()>,
) -> Element {
    let mut shapes = use_signal(Vec::<SVGElement>::new);

    let mut svg_dimensions = use_signal(|| None as Option<PixelsSize>);

    let mut mouse_pos = use_signal(|| (0., 0.));

    if let Some(size) = svg_dimensions()
        && let Some((geobox, nwr)) = osm_data
    {
        debug!(
            "OSMData size: {} for a box of {}x{} km",
            nwr.len(),
            geobox.width_km(),
            geobox.height_km()
        );
        shapes.set(generate_shapes(size, &geobox, &nwr));
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
