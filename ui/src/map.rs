use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc, f64::consts::PI};
use wasm_bindgen::closure::Closure;
use web_sys::{wasm_bindgen::JsCast as _, CanvasRenderingContext2d};

const MAP_CSS: Asset = asset!("/assets/styling/map.css");
const CANVAS_ID: &str = "map-canvas";

// FIXME unwraps
fn create_context() -> CanvasRenderingContext2d {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id(CANVAS_ID)
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap()
}

// I wonder how faster could a custom impl be
fn render_map(ctx: &CanvasRenderingContext2d, offset: (f64, f64), center: (f64, f64)) {
    ctx.begin_path();
    ctx.set_stroke_style_str("#00FFA1");

    ctx.move_to(center.0 + offset.0, center.1 + offset.1);

    for i in 0..101 {
        let angle: f64 = PI * 2. * 0.01 * i as f64;
        let pt = (center.0 + 100., center.1);
        let newx = center.0 + angle.cos() * (pt.0 - center.0) - angle.sin() * (pt.1 - center.1);
        let newy = center.1 + angle.sin() * (pt.0 - center.0) + angle.cos() * (pt.1 - center.1);
        if i == 0 {
            ctx.move_to(newx + offset.0, newy + offset.1);
        } else {
            ctx.line_to(newx + offset.0, newy + offset.1);
        }
    }
    // ctx.close_path();
    ctx.stroke();
}

#[component]
pub fn Map() -> Element {
    debug!("Hi :3");

    let window = web_sys::window().unwrap();
    let window_size = (
        window.inner_width().ok().and_then(|s|s.as_f64()).unwrap(),
        window.inner_height().ok().and_then(|s|s.as_f64()).unwrap(),
    );

    let mut mouse_position = use_signal(|| (0., 0.));
    let mut is_dragging = use_signal(|| false);
    
    use_effect(move || {
        let update_fn = Rc::new(RefCell::new(None));

        fn request_animation_frame(update_fn: &Closure<dyn FnMut()>) {
            web_sys::window()
                .unwrap()
                .request_animation_frame(update_fn.as_ref().unchecked_ref())
                .unwrap();
        }

        let glctx = create_context();

        *update_fn.borrow_mut() = Some(Closure::wrap(Box::new({
            let glctx = glctx.clone();
            let canvas = glctx.canvas().unwrap();
            let update_fn = update_fn.clone();
            let document = web_sys::window().unwrap().document().unwrap();
            move || {
                // Without this, any tries at fetching any of use_signal s above, will result in a panic
                // Which isn't critical, but ugly
                if document.get_element_by_id(CANVAS_ID).is_none() {
                    return;
                }

                glctx.clear_rect(0., 0., canvas.width() as f64, canvas.height() as f64);


                // FIXME this should be removed since we place the canvas at 0x0y
                let canvas_offset = {
                    let aabb = canvas.get_bounding_client_rect();
                    (-aabb.x(), -aabb.y())
                };

                let circle_center = (
                    mouse_position().0,
                    mouse_position().1,
                );

                render_map(&glctx, canvas_offset, circle_center);
                request_animation_frame(update_fn.borrow().as_ref().unwrap())
            }
        }) as Box<dyn FnMut()>));

        request_animation_frame(update_fn.borrow().as_ref().unwrap());
    });

    rsx! {
        document::Link { rel: "stylesheet", href: MAP_CSS }

        canvas {
            id: CANVAS_ID,
            width: window_size.0, height: window_size.1,

            onmousedown: move |_| {
                debug!("Mouse down on canvas!");
                is_dragging.set(true)
            },
            onclick: move |_| {
                debug!("Canvas clicked!");
            },
            onmouseup: move |_| {
                debug!("Mouse up");
                is_dragging.set(false)
            },
            onmousemove: move |event| {
                let x = event.data.client_coordinates().x;
                let y = event.data.client_coordinates().y;
                // debug!("Move {x},{y}");
                mouse_position.set((x, y));
            },
        }
    }
}
