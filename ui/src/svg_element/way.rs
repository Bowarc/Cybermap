use osm::{
    coord::{geo::GeoPoint, screen::ScreenPoint},
    element::OSMWay,
};

use crate::svg_element::SVGElement;

const COLOR: &str = "rgba(0, 255, 255, 0.33)";
const OUTLINE_COLOR: &str = "rgba(0, 255, 255, 1)";

pub const WAY_WIDTH_KM: f64 = 0.008;
const WAY_BORDER_WIDTH_KM: f64 = 0.002;

pub fn gen_way(
    way: &OSMWay,
    roads: &mut Vec<SVGElement>,
    map_pt: impl Fn(GeoPoint) -> ScreenPoint,
    scale_km: f64,
) {
    let mut outlines: [Vec<ScreenPoint>; 2] = Default::default();

    let way_width_scaled = WAY_WIDTH_KM * scale_km;
    let way_border_distance_scaled = way_width_scaled * 0.5;

    for i in 1..way.nodes.len() {
        let last = way.nodes.get(i - 1).unwrap();
        let Some(current) = way.nodes.get(i) else {
            break;
        };

        let p1 = map_pt(last.pos);
        let p2 = map_pt(current.pos);

        let rotate_pt = |origin: &ScreenPoint, distance: f64, angle: f64| -> ScreenPoint {
            let pt = ScreenPoint::new(origin.x + distance, origin.y);
            ScreenPoint::new(
                origin.x + angle.cos() * (pt.x - origin.x) - angle.sin() * (pt.y - origin.y),
                origin.y + angle.sin() * (pt.x - origin.x) + angle.cos() * (pt.y - origin.y),
            )
        };

        let line_angle = (p2.y - p1.y).atan2(p2.x - p1.x);

        outlines[0].extend([
            rotate_pt(
                &p1,
                way_border_distance_scaled,
                line_angle + 90f64.to_radians(),
            ),
            rotate_pt(
                &p2,
                way_border_distance_scaled,
                line_angle + 90f64.to_radians(),
            ),
        ]);

        outlines[1].extend([
            rotate_pt(
                &p1,
                way_border_distance_scaled,
                line_angle + -90f64.to_radians(),
            ),
            rotate_pt(
                &p2,
                way_border_distance_scaled,
                line_angle + -90f64.to_radians(),
            ),
        ]);
    }

    // Road
    roads.push(SVGElement::Polyline {
        points: way
            .nodes
            .iter()
            .map(|node| map_pt(node.pos))
            .map(|pt| (pt.x as u32, pt.y as u32))
            .collect::<Vec<_>>(),
        width: way_width_scaled as u32,
        color: COLOR.to_string(),
        fill: String::from("none"),
    });

    // for pt in outlines[0].iter() {
    //     shapes.push(Shape::Rect {
    //         x: pt.x as u32,
    //         y: pt.y as u32,
    //         width: 5,
    //         height: 5,
    //         color: String::from("red"),
    //     })
    // }
    // 
    // for pt in outlines[1].iter() {
    //     shapes.push(Shape::Rect {
    //         x: pt.x as u32,
    //         y: pt.y as u32,
    //         width: 5,
    //         height: 5,
    //         color: String::from("green"),
    //     })
    // }

    for outline in outlines {
        roads.push(SVGElement::Polyline {
            points: outline
                .iter()
                .map(|pt| (pt.x as u32, pt.y as u32))
                .collect::<Vec<_>>(),
            width: (WAY_BORDER_WIDTH_KM * scale_km) as u32,
            color: OUTLINE_COLOR.to_string(),
            fill: String::from("none"),
        });
    }
}
