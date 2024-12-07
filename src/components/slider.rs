use leptos::{logging::log, prelude::*};

#[component]
pub fn Slider(on_change: Box<dyn Fn(u8)>) -> impl IntoView {
    view! {
        <input
            type="range"
            min="0"
            max="100"
            on:change=move |ev| {
                log!("changed!");
                on_change(event_target_value(&ev).parse::<u8>().unwrap_or(0))
            }
        />
    }
}
