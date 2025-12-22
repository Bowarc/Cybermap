use dioxus::{html::geometry::PixelsSize, prelude::*};
use osm::{coord::{geo::{GeoBox, GeoPoint}, screen::ScreenPoint}, element::NWR};
use reqwest::Client;

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
        }
    }
}

struct Item {
    x: u32,
    y: u32,

    width: u32,
    height: u32,

    color: &'static str,
}

impl Item {
    fn to_element(&self) -> Element {
        rsx! {
            rect {
                x: self.x,
                y: self.y,
                width: self.width,
                height: self.height,
                fill: self.color
            }
        }
    }
}

fn generate_shapes(svg_size: PixelsSize, bx: &GeoBox, nwr: &[NWR]) -> Vec<Shape> {
    let map_pt = |pt: GeoPoint| -> ScreenPoint {
        ScreenPoint {
            x: svg_size.width * ((pt.lon() - bx.min().lon()) / bx.width()),
            y: svg_size.height * (1. - (pt.lat() - bx.min().lat()) / (bx.height())),
        }
    };

    let mut shapes = Vec::new();

    for node in nwr.iter() {
        match node {
            NWR::Node(node) => {
                let pos = map_pt(node.pos);
                shapes.push(Shape::Rect {
                    x: pos.x as u32,
                    y: pos.y as u32,
                    width: 3,
                    height: 3,
                    color: String::from("red"),
                });
            }
            NWR::Way(_node) => {
                // No Ways in the test data
            }
            _ => (),
        }
    }

    shapes
}

#[component]
pub fn Map() -> Element {
    let mut shapes = use_signal(Vec::<Shape>::new);

    let mut mouse_pos = use_signal(|| (0., 0.));

    let items = &[
        Item {
            x: (mouse_pos().0 as u32).saturating_sub(50),
            y: (mouse_pos().1 as u32).saturating_sub(50),
            width: 100,
            height: 100,
            color: "lightblue",
        },
        Item {
            x: 10,
            y: 200,
            width: 50,
            height: 100,
            color: "red",
        },
    ];

    let build_shapes = async |svg_size: PixelsSize, shapes: &mut Signal<Vec<Shape>>| {
        let bx = {
            let lat = todo!("Redacted");
            let lon = todo!("Redacted");

            let box_center = GeoPoint::new(lat, lon);
            let range_km = 0.5;

            let scale_factor = range_km / svg_size.width.max(svg_size.height);

            let box_size = (
                svg_size.width * scale_factor,
                svg_size.height * scale_factor,
            );

            GeoBox::from_center_and_size(box_center, box_size)
        };
        let query = format!(
            r#"
            [out:json][timeout:360][bbox:{},{},{},{}];
            nwr["amenity"="restaurant"];
            out center;"#,
            bx.min().lat(),
            bx.min().lon(),
            bx.max().lat(),
            bx.max().lon()
        );

        let client = Client::new();
        let json_value = client
            .get("http://127.0.0.1:42061/overpass_api")
            .query(&[("data", query)])
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        let nwr = osm::parsing::parse_osm_json(json_value).unwrap();

        shapes.set(generate_shapes(svg_size, &bx, &nwr));
    };

    let mut dimensions = use_signal(|| None as Option<PixelsSize>);

    debug!("Map svg render");

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS }

        div {
            "This element is {dimensions():.1?}"
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
                debug!("MAP RESIZED: {}x{}", size.width, size.height);
                dimensions.set(Some(size));
                build_shapes(size, &mut shapes).await
            }},

            for item in items.iter() {{
                item.to_element()
            }}

            for shape in shapes.iter(){{
                shape.to_element()
            }}
        }
    }
}
