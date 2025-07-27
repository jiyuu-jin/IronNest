use {
    crate::components::mish::{
        json_editor::JsonEditor, number_editor::NumberEditor, text_editor::TextEditor,
    },
    leptos::prelude::*,
    std::sync::Arc,
};

#[component]
pub fn Editor(
    state: serde_json::Value,
    action: impl Fn(Vec<u8>) + 'static + Send + Sync,
) -> impl IntoView {
    let action = Arc::new(action);
    let raw_editor_mode = RwSignal::new(false);

    let editor = move || {
        let state = state.clone();
        if raw_editor_mode.get() {
            let action = action.clone();
            view! { <JsonEditor state=Some(state) set_config_server_action=move |s| { action(s) } /> }.into_any()
        } else {
            let action = action.clone();
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
                    view! { <NumberEditor state=n.to_string() set_config_server_action=move |s| { action(s) } /> }
                        .into_any()
                }
                serde_json::Value::Array(a) => {
                    let action = Arc::new(action);
                    a.iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let a = a.clone();
                            let action = action.clone();
                            view! {
                                <p>Array: {i}</p>
                                <div style="border-left: 5px solid black">
                                    <NestedEditor
                                        state=v.clone()
                                        action=Box::new(move |s| {
                                            let mut a = a.clone();
                                            a[i] = serde_json::from_slice(&s).unwrap();
                                            action(serde_json::to_vec(&a).unwrap());
                                        })
                                    />
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                }
                serde_json::Value::Object(o) => {
                    let action = Arc::new(action);
                    o.clone()
                        .into_iter()
                        .map(|(k, v)| {
                            let o = o.clone();
                            let action = action.clone();
                            view! {
                                <div>Key: {k.clone()}</div>
                                <div style="border-left: 5px solid black">
                                    <NestedEditor
                                        state=v.clone()
                                        action=Box::new(move |s| {
                                            let mut o = o.clone();
                                            o.insert(k.clone(), serde_json::from_slice(&s).unwrap());
                                            action(serde_json::to_vec(&o).unwrap());
                                        })
                                    />
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                        .into_any()
                }
                _ => view! { <JsonEditor state=Some(state) set_config_server_action=move |s| { action(s) } /> }
                    .into_any(),
            }
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

#[component]
pub fn NestedEditor(
    state: serde_json::Value,
    action: Box<dyn Fn(Vec<u8>) + 'static + Send + Sync>,
) -> impl IntoView {
    view! { <Editor state=state action=action /> }
}
