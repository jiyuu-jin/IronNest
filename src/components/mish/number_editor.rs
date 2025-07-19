use leptos::prelude::*;

#[component]
pub fn NumberEditor(
    state: String,
    set_config_server_action: impl Fn(Vec<u8>) + 'static,
) -> impl IntoView {
    let (state, set_state) = signal(state);
    view! {
        <p>"Number editor"</p>
        <textarea on:input=move |ev| {
            set_state.set(event_target_value(&ev));
        }>{state}</textarea>
        <button on:click=move |_| {
            let s = state.get();
            web_sys::console::log_1(&format!("state: {s:?}").into());
            let s = serde_json::from_str::<serde_json::Value>(&s);
            if let Ok(s) = s {
                if s.is_number() {
                    set_config_server_action(serde_json::to_vec(&s).unwrap());
                } else {
                    web_sys::window()
                        .unwrap()
                        .alert_with_message(&format!("State is not a number: {s:?}"))
                        .unwrap();
                }
            } else {
                web_sys::window()
                    .unwrap()
                    .alert_with_message(&format!("Error parsing number: {:?}", s))
                    .unwrap();
            }
        }>"Save"</button>
    }
}
