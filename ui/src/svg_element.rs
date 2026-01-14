use dioxus::{
    core::Element,
    html::geometry::PixelsSize,
    prelude::{debug, dioxus_core, dioxus_elements, dioxus_signals, rsx},
};

use osm::{
    coord::{
        geo::{GeoBox, GeoPoint},
        screen::ScreenPoint,
    },
    element::{NWR, OSMNode, OSMWay},
};

mod building;
mod node;
mod way;

use building::gen_building;
use node::{gen_crosswalk, gen_traficlight};
use way::gen_way;

pub enum SVGElement {
    Rect {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: String,
    },
    Line {
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        width: u32,
        color: String,
    },
    Polyline {
        points: Vec<(u32, u32)>,
        width: u32,
        color: String,
        fill: String,
    },
}

impl SVGElement {
    pub fn to_element(&self) -> Element {
        match self {
            #[rustfmt::skip]
            Self::Rect { x, y, width, height, color } => {
                rsx! {
                    rect { x: *x, y: *y, width: *width, height: *height, fill: color.clone() }
                }
            }
            #[rustfmt::skip]
            Self::Line { x1, y1, x2, y2, width, color } => {
                rsx! {
                    line { x1: *x1, y1: *y1, x2: *x2, y2: *y2, stroke_width: *width, stroke: color.clone() }
                }
            }
            #[rustfmt::skip]
            Self::Polyline { points, width, color, fill } => {
                rsx! {
                    polyline {
                        points: points.iter().fold(String::new(), |mut s, (x, y)|{
                            use std::fmt::Write as _;
                            s.write_str(&format!("{x},{y} ")).unwrap();
                            s
                        }),
                        stroke_width: *width,
                        stroke: color.clone(),
                        fill: fill.clone(),
                    }

                }
            }
        }
    }
}

pub fn generate_from_osm(svg_size: PixelsSize, bx: &GeoBox, nwr: &NWR) -> Vec<SVGElement> {
    let map_pt = |pt: GeoPoint| -> ScreenPoint {
        ScreenPoint {
            x: svg_size.width * ((pt.lon() - bx.min().lon()) / (bx.max().lon() - bx.min().lon())),
            y: svg_size.height
                * (1. - (pt.lat() - bx.min().lat()) / (bx.max().lat() - bx.min().lat())),
        }
    };

    let scale_km = svg_size.width / bx.width_km(); // Used to scale real world sizes to the viewport
    debug!("Scale km: {scale_km}");

    let mut shapes = Vec::new();
    let mut roads = Vec::new();
    let mut buildings = Vec::new();
    // let mut land = Vec::new();

    for (_, node) in nwr.nodes.iter() {
        if node.tags.get("highway").map(|s| &**s) == Some("crossing") {
            crate::svg_element::gen_crosswalk(node, &mut shapes, map_pt, scale_km);
        } else if node.tags.get("highway").map(|s| &**s) == Some("traffic_signals") {
            crate::svg_element::gen_traficlight(node, &mut shapes, map_pt, scale_km);
        };
    }

    for (_, way) in nwr.ways.iter() {
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
        if way.tags.contains_key("highway") && !way.tags.contains_key("area") {
            gen_way(way, &mut roads, map_pt, scale_km);
            continue;
        }

        if way.tags.contains_key("building") && way.tags.get("wall").map(|s| &**s) != Some("no") {
            gen_building(way, &mut buildings, map_pt, scale_km);
        }
    }

    roads.extend(shapes);
    roads.extend(buildings);
    roads
}
