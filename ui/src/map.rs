use dioxus::{
    html::geometry::PixelsSize,
    prelude::*,
};

const MAP_CSS: Asset = asset!("/assets/styling/map.css");
const CANVAS_ID: &str = "map-canvas";

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

#[component]
pub fn Map() -> Element {
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
            id: CANVAS_ID,
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
            onresize: move |cx| 'onresize: {
                let size = match cx.data().get_content_box_size() {
                    Ok(size) => size,
                    Err(e) => {
                        error!("Failed to unpack map's svg onresize event due to: {e}");
                        break 'onresize
                    }
                };

                debug!("MAP RESIZED: {}x{}", size.width, size.height);
                dimensions.set(Some(size));
            },

            for item in items.iter() {{
                item.to_element()
            }}
        }
    }
}
