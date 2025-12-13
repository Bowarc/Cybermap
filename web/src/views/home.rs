use dioxus::prelude::*;
use ui::Hero;

#[component]
pub fn Home() -> Element {
    rsx! {
        button {
            // The `onclick` event accepts a closure with the signature `fn(Event)`
            onclick: move |event_data| println!("clicked! I got the event data: {event_data:?}"),
            "Click meh"
        }
        Hero {}

    }
}
