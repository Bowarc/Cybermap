use osm::{
    coord::{geo::GeoPoint, screen::ScreenPoint},
    element::OSMWay,
};

use crate::svg_element::SVGElement;

const COLOR: &str = "rgba(0, 255, 255, 0.33)";
const OUTLINE_COLOR: &str = "rgba(0, 255, 255, 1)";

// FIXME:
// Not all roads have the same width,
// Using a fixed size (8m here) is cusing some small inaccuracies in relation to other unrelated elements (buildings)
pub const WAY_WIDTH_KM: f64 = 0.008;

const WAY_BORDER_WIDTH_KM: f64 = 0.002;

// TODO:
// Find a way to merge roads into others
// Currently they ignore each others and overlap
// It's bad
pub fn gen_way(
    way: &OSMWay,
    roads: &mut Vec<SVGElement>,
    map_pt: impl Fn(GeoPoint) -> ScreenPoint,
    scale_km: f64,
) {
    let mut outlines: [Vec<ScreenPoint>; 2] = Default::default();

    let way_width_scaled = WAY_WIDTH_KM * scale_km;
    let way_width_scaled_half = way_width_scaled * 0.5;
    for i in 1..way.nodes.len() + 1 {
        let Some(last) = way.nodes.get(i - 1) else {
            break;
        };
        let current = match way.nodes.get(i) {
            Some(current) => current,
            // Circular ways
            None if way.nodes.first() == way.nodes.last() && i > 1 => way.nodes.get(1).unwrap(),
            None => break,
        };

        let last_pt = map_pt(last.pos);
        let current_pt = map_pt(current.pos);

        let rotate_pt = |origin: &ScreenPoint, distance: f64, angle: f64| -> ScreenPoint {
            let pt = ScreenPoint::new(origin.x + distance, origin.y);
            ScreenPoint::new(
                origin.x + angle.cos() * (pt.x - origin.x) - angle.sin() * (pt.y - origin.y),
                origin.y + angle.sin() * (pt.x - origin.x) + angle.cos() * (pt.y - origin.y),
            )
        };

        let line_angle = (current_pt.y - last_pt.y).atan2(current_pt.x - last_pt.x);

        clamp_and_push(
            [
                rotate_pt(
                    &last_pt,
                    way_width_scaled_half,
                    line_angle + 90f64.to_radians(),
                ),
                rotate_pt(
                    &current_pt,
                    way_width_scaled_half,
                    line_angle + 90f64.to_radians(),
                ),
            ],
            &mut outlines[0],
        );
        clamp_and_push(
            [
                rotate_pt(
                    &last_pt,
                    way_width_scaled_half,
                    line_angle - 90f64.to_radians(),
                ),
                rotate_pt(
                    &current_pt,
                    way_width_scaled_half,
                    line_angle - 90f64.to_radians(),
                ),
            ],
            &mut outlines[1],
        );
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

pub fn line_intersection(l1: &[&ScreenPoint; 2], l2: &[&ScreenPoint; 2]) -> Option<ScreenPoint> {
    let (p1, p2) = (l1[0], l1[1]);
    let (p3, p4) = (l2[0], l2[1]);

    // Denominator (if zero, lines are parallel)
    let denom = (p4.y - p3.y) * (p2.x - p1.x) - (p4.x - p3.x) * (p2.y - p1.y);
    if denom.abs() < 1e-10 {
        return None;
    }

    let ua = ((p4.x - p3.x) * (p1.y - p3.y) - (p4.y - p3.y) * (p1.x - p3.x)) / denom;
    let ub = ((p2.x - p1.x) * (p1.y - p3.y) - (p2.y - p1.y) * (p1.x - p3.x)) / denom;

    // Check if intersection is within line segments
    if (0.0..=1.0).contains(&ua) && (0.0..=1.0).contains(&ub) {
        Some(ScreenPoint {
            x: (p1.x + ua * (p2.x - p1.x)),
            y: (p1.y + ua * (p2.y - p1.y)),
        })
    } else {
        None // Intersection outside line segments
    }
}

fn clamp_and_push(mut current_line: [ScreenPoint; 2], others: &mut Vec<ScreenPoint>) {
    let clamp_one = |current: &mut [ScreenPoint; 2], other: [&mut ScreenPoint; 2]| {
        // line_interaction already checks for if the lines are actually crossing
        // if !line_line(&[&current[0], &current[1]], &[other[0], other[1]]) {
        //     return;
        // }

        let Some(intersection_pt) =
            line_intersection(&[&current[0], &current[1]], &[other[0], other[1]])
        else {
            return;
        };

        current[0] = intersection_pt;

        other[0].x = intersection_pt.x;
        other[0].y = intersection_pt.y;
    };

    let others_len = others.len();

    // NOTE: Merging more than the last 3 lines could result in roads that overlap themselves (like spirals)
    // to weirdly merge at abnormal places
    //
    // Iteration:
    // At first i wanted to iter over the last 3 lines from oldest to newest, so:
    // len - 5, len - 6
    // len - 3, len - 4
    // len - 1, len - 2
    //
    // Then I figured checking made up lines in between could also be helpful (which it was)
    // len - 5, len - 6
    // len - 4, len - 5
    // len - 3, len - 4
    // len - 2, len - 3
    // len - 1, len - 2
    for line_index in (1..=5).rev() {
        if others_len < line_index + 1 {
            continue;
        }
        let Ok(previous_line) =
            others.get_disjoint_mut([others_len - line_index, others_len - (line_index + 1)])
        else {
            continue;
        };

        clamp_one(&mut current_line, previous_line);
    }

    others.extend(current_line);

    // Maybe helpful, remove useless nodes
    others.dedup();
}
