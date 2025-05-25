use {crate::components::mish::mish_state_page::SetMishState, leptos::prelude::*};

#[component]
pub fn RawEditor(
    name: String,
    state: Option<serde_json::Value>,
    set_config_server_action: ServerAction<SetMishState>,
) -> impl IntoView {
    let (state, set_state) = signal(
        state
            .map(|s| serde_json::to_string_pretty(&s).unwrap())
            .unwrap_or_default(),
    );
    view! {
        <p>"Raw editor"</p>
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
            match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(s) => {
                    set_config_server_action
                        .dispatch(SetMishState {
                            name: name.clone(),
                            state: serde_json::to_string(&s).unwrap(),
                        });
                }
                Err(e) => {
                    web_sys::window()
                        .unwrap()
                        .alert_with_message(&format!("Error parsing JSON: {:?}", e))
                        .unwrap();
                }
            }
        }>"Save"</button>
    }
}
