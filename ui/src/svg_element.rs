use dioxus::{
    core::Element,
    prelude::{dioxus_core, dioxus_elements, dioxus_signals, rsx},
};

mod node;
mod way;
pub use node::{gen_crosswalk, gen_traficlight};
pub use way::gen_way;

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
