use {leptos::prelude::*, leptos_use::signal_throttled};

#[component]
pub fn ColorPicker(
    label: String,
    default_value: String,
    on_change: Box<dyn Fn(String)>,
) -> impl IntoView {
    let (value, set_value) = signal(None);
    let value_throttled: Signal<Option<String>> = signal_throttled(value, 500.);
    Effect::new(move |_| {
        if let Some(value) = value_throttled.get() {
            on_change(value);
        }
    });

    view! {
        <div>
            <input
                type="color"
                id="colorPicker"
                name="colorPicker"
                value=default_value
                on:input=move |ev| {
                    leptos::logging::log!("changed color");
                    let hex_value = event_target_value(&ev);
                    leptos::logging::log!("hex_value: {hex_value}");
                    set_value.set(Some(hex_value));
                }
            />

            <label for="colorPicker">{label}</label>

        // {move || error.get().map(|error| view! { <p class="text-red-500">{error}</p> })}
        </div>
    }
}
