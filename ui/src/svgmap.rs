use std::rc::Rc;

use dioxus::{html::geometry::PixelsSize, prelude::*};
use osm::{
    coord::{
        geo::{GeoBox, GeoPoint},
        screen::ScreenPoint,
    },
    element::NWR,
};

const MAP_CSS: Asset = asset!("/assets/styling/map.css");
const SVG_ID: &str = "map-svg";

pub enum Shape {
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

impl Shape {
    fn to_element(&self) -> Element {
        match self {
            Shape::Rect {
                x,
                y,
                width,
                height,
                color,
            } => {
                rsx! {
                    rect {
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        fill: color.clone()
                    }
                }
            }
            Shape::Line {
                x1,
                y1,
                x2,
                y2,
                width,
                color,
            } => {
                rsx! {
                    line {

                        x1: *x1,
                        y1: *y1,
                        x2: *x2,
                        y2: *y2,
                        stroke_width: *width,
                        stroke: color.clone()
                    }
                }
            }
            Shape::Polyline {
                points,
                width,
                color,
                fill,
            } => {
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

fn generate_rectangle_points(p1: &ScreenPoint, p2: &ScreenPoint, width: f64) -> [ScreenPoint; 4] {
    // Calculate the vector between the two points
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;

    // Calculate the perpendicular vector (rotated 90 degrees)
    let perpendicular_dx = -dy;
    let perpendicular_dy = dx;

    // Normalize the perpendicular vector
    let magnitude = (perpendicular_dx.powi(2) + perpendicular_dy.powi(2)).sqrt();
    let unit_perpendicular_x = perpendicular_dx / magnitude;
    let unit_perpendicular_y = perpendicular_dy / magnitude;

    // Scale the perpendicular vector to the width
    let scaled_perpendicular_x = unit_perpendicular_x * width;
    let scaled_perpendicular_y = unit_perpendicular_y * width;

    // Calculate the 4 points of the rectangle
    [
        ScreenPoint {
            x: p1.x + scaled_perpendicular_x,
            y: p1.y + scaled_perpendicular_y,
        },
        ScreenPoint {
            x: p2.x + scaled_perpendicular_x,
            y: p2.y + scaled_perpendicular_y,
        },
        ScreenPoint {
            x: p2.x - scaled_perpendicular_x,
            y: p2.y - scaled_perpendicular_y,
        },
        ScreenPoint {
            x: p1.x - scaled_perpendicular_x,
            y: p1.y - scaled_perpendicular_y,
        },
    ]
}

fn generate_shapes(svg_size: PixelsSize, bx: &GeoBox, nwr: &[NWR]) -> Vec<Shape> {
    let map_pt = |pt: GeoPoint| -> ScreenPoint {
        ScreenPoint {
            x: svg_size.width * ((pt.lon() - bx.min().lon()) / bx.width()),
            y: svg_size.height * (1. - (pt.lat() - bx.min().lat()) / (bx.height())),
        }
    };

    let mut shapes = Vec::new();
    let mut roads = Vec::new();

    for node in nwr.iter() {
        match node {
            NWR::Node(node) => {
                let pos = map_pt(node.pos);

                let color = if node.tags.contains_key("crossing") {
                    String::from("white")
                } else {
                    String::from("red")
                };
                shapes.push(Shape::Rect {
                    x: pos.x as u32,
                    y: pos.y as u32,
                    width: 3,
                    height: 3,
                    color,
                });
            }
            NWR::Way(way) => {
                if way.nodes.is_empty() {
                    debug!("Skipping nodeless way: {way:?}");
                    continue;
                }
                if way.tags.contains_key("area") {
                    debug!("Skipping area: {way:?}");
                    continue;
                }
                let color = String::from("rgba(0, 255, 255, 0.33)");
                let outline_color = String::from("rgba(255, 0, 255, 1)");

                let mut outlines: [Vec<ScreenPoint>; 2] = Default::default();

                for i in 1..way.nodes.len() {
                    let last = way.nodes.get(i - 1).unwrap();
                    let Some(current) = way.nodes.get(i) else {
                        break;
                    };

                    let p1 = map_pt(last.pos);
                    let p2 = map_pt(current.pos);

                    let rect = generate_rectangle_points(&p1, &p2, 5.);

                    outlines[0].extend([
                        ScreenPoint::new(rect[0].x, rect[0].y),
                        ScreenPoint::new(rect[1].x, rect[1].y),
                    ]);

                    // FIXME: There is some artifcats caused by this line,
                    // Some outlines are matching with i-don't-know-what node from the same road
                    // I don't understand why this is only with one side of the outline (always the green one)     
                    outlines[1].extend([
                        ScreenPoint::new(rect[2].x, rect[2].y),
                        ScreenPoint::new(rect[3].x, rect[3].y),
                    ]);
                }

                // Road
                roads.push(Shape::Polyline {
                    points: way
                        .nodes
                        .iter()
                        .map(|node| map_pt(node.pos))
                        .map(|pt| (pt.x as u32, pt.y as u32))
                        .collect::<Vec<_>>(),
                    width: 2,
                    color,
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
                    roads.push(Shape::Polyline {
                        points: outline
                            .iter()
                            .map(|pt| (pt.x as u32, pt.y as u32))
                            .collect::<Vec<_>>(),
                        width: 1,
                        color: outline_color.clone(),
                        fill: String::from("none"),
                    });
                }
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
    let mut shapes = use_signal(Vec::<Shape>::new);

    let mut svg_dimensions = use_signal(|| None as Option<PixelsSize>);

    let mut mouse_pos = use_signal(|| (0., 0.));

    if let Some(size) = svg_dimensions()
        && let Some((geobox, nwr)) = osm_data
    {
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
