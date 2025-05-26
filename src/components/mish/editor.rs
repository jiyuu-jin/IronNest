use {
    crate::components::mish::{
        json_editor::JsonEditor, number_editor::NumberEditor, text_editor::TextEditor,
    },
    leptos::prelude::*,
};

#[component]
pub fn Editor(state: serde_json::Value, action: impl Fn(Vec<u8>) + 'static) -> impl IntoView {
    let raw_editor_mode = RwSignal::new(false);

    let editor = if raw_editor_mode.get() {
        view! { <JsonEditor state=Some(state) set_config_server_action=action /> }.into_any()
    } else {
        match state {
            serde_json::Value::Bool(b) => view! {
                <input
                    type="checkbox"
                    checked=b
                    on:input:target=move |ev| {
                        let value = ev.target().checked();
                        action(serde_json::to_vec(&value).unwrap());
                    }
                />
            }
            .into_any(),
            serde_json::Value::String(s) => view! {
                <TextEditor
                    state=s
                    set_config_server_action=move |s| { action(serde_json::to_vec(&s).unwrap()) }
                />
            }
            .into_any(),
            serde_json::Value::Number(n) => {
                view! { <NumberEditor state=n.to_string() set_config_server_action=action /> }
                    .into_any()
            }
            _ => view! { <JsonEditor state=Some(state) set_config_server_action=action /> }
                .into_any(),
        }
    };

    view! {
        <div>
            <label for="raw-editor-mode">"RAW editor mode"</label>
            <input type="checkbox" id="raw-editor-mode" bind:checked=raw_editor_mode />
        </div>
        {editor}
    }
}
