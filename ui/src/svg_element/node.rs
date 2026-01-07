use osm::{
    coord::{geo::GeoPoint, screen::ScreenPoint},
    element::OSMNode,
};

use crate::svg_element::SVGElement;

const CROSSWALK_SIZE_KM: f64 = super::way::WAY_WIDTH_KM * 0.5;
const CROSSWALK_COLOR: &str = "rgba(255, 255, 255, 1)";

const TRAFICLIGHT_SIZE_KM: f64 = super::way::WAY_WIDTH_KM * 0.3;
const TRAFICLIGHT_COLOR: &str = "rgba(255, 0, 0, 1)";

pub fn gen_crosswalk(
    node: &OSMNode,
    nodes: &mut Vec<SVGElement>,
    map_pt: impl Fn(GeoPoint) -> ScreenPoint,
    scale_km: f64,
) {
    let pos = map_pt(node.pos);
    let crosswalk_width_scaled = CROSSWALK_SIZE_KM * scale_km;

    nodes.push(SVGElement::Rect {
        x: (pos.x - crosswalk_width_scaled * 0.5) as u32,
        y: (pos.y - crosswalk_width_scaled * 0.5) as u32,
        width: crosswalk_width_scaled as u32,
        height: crosswalk_width_scaled as u32,
        color: CROSSWALK_COLOR.to_string(),
    });
}

pub fn gen_traficlight(
    node: &OSMNode,
    nodes: &mut Vec<SVGElement>,
    map_pt: impl Fn(GeoPoint) -> ScreenPoint,
    scale_km: f64,
) {
    let pos = map_pt(node.pos);
    let traficlight_width_scaled = TRAFICLIGHT_SIZE_KM * scale_km;

    nodes.push(SVGElement::Rect {
        x: (pos.x - traficlight_width_scaled * 0.5) as u32,
        y: (pos.y - traficlight_width_scaled * 0.5) as u32,
        width: traficlight_width_scaled as u32,
        height: traficlight_width_scaled as u32,
        color: TRAFICLIGHT_COLOR.to_string(),
    });
}
