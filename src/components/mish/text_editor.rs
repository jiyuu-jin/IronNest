use leptos::prelude::*;

#[component]
pub fn TextEditor(
    state: String,
    set_config_server_action: impl Fn(String) + 'static,
) -> impl IntoView {
    let (state, set_state) = signal(state);
    view! {
        <p>"Text editor"</p>
        <textarea
            on:input=move |ev| {
                set_state.set(event_target_value(&ev));
            }
            style="height: 200px; width: 100%;"
        >
            {state}
        </textarea>
        <button on:click=move |_| {
            let s = state.get();
            web_sys::console::log_1(&format!("state: {:?}", s).into());
            set_config_server_action(s);
        }>"Save"</button>
    }
}
