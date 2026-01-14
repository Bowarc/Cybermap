use super::SVGElement;
use osm::{
    coord::{geo::GeoPoint, screen::ScreenPoint},
    element::OSMWay,
};

const BUILDING_BORDER_WIDTH_KM: f64 = 0.002;
pub fn gen_building(
    way: &OSMWay,
    buildings: &mut Vec<SVGElement>,
    map_pt: impl Fn(GeoPoint) -> ScreenPoint,
    scale_km: f64,
) {
    // TODO Handle that
    if way.nodes.first() != way.nodes.last() {
        return;
    }

    let building_wall_width_scaled = (BUILDING_BORDER_WIDTH_KM * scale_km) as u32;
    let pts = way
        .nodes
        .iter()
        .map(|node| map_pt(node.pos))
        .map(|pt| (pt.x as u32, pt.y as u32))
        .collect::<Vec<_>>();

    buildings.push(SVGElement::Polyline {
        points: pts,
        width: building_wall_width_scaled,
        color: String::from("red"),
        fill: String::from("rgba(255, 0, 0, 0.5)"),
    })
}
