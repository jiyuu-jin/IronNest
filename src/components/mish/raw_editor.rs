use {crate::components::pages::mish_state_page::SetMishState, leptos::prelude::*};

#[component]
pub fn RawEditor(
    name: String,
    state: String,
    set_config_server_action: ServerAction<SetMishState>,
) -> impl IntoView {
    let (state, set_state) = signal(state);
    view! {
        <textarea on:input=move |ev| {
            set_state.set(event_target_value(&ev));
        }>{state}</textarea>
        <button on:click=move |_| {
            let s = state.get();
            web_sys::console::log_1(&format!("state: {:?}", s).into());
            set_config_server_action
                .dispatch(SetMishState {
                    name: name.clone(),
                    state: s,
                });
        }>"Save"</button>
    }
}
