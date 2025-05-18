use {
    crate::components::{
        layout::{Toast, ToastContext},
        mish::{raw_editor::RawEditor, text_editor::TextEditor},
    },
    leptos::prelude::*,
    leptos_router::{hooks::use_params, params::Params},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MishState {
    name: String,
    state: serde_json::Value,
}

#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
pub struct MishStateRow {
    name: String,
    state: serde_json::Value,
}

#[server(GetMishState)]
async fn get_mish_state(name: String) -> Result<Option<MishState>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let mish_state = get_mish_state_query(&pool, &name).await?;
    Ok(mish_state)
}

#[cfg(feature = "ssr")]
pub async fn get_mish_state_query(
    pool: &sqlx::PgPool,
    name: &str,
) -> Result<Option<MishState>, sqlx::Error> {
    // #[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
    // struct Row {
    //     name: String,
    //     state: String,
    // }
    let query = "
        SELECT name, state
        FROM mish_states
        WHERE name = $1
    ";
    sqlx::query_as::<_, MishStateRow>(query)
        .bind(name)
        .fetch_optional(pool)
        .await
        .map(|row| {
            row.map(|row| MishState {
                name: row.name,
                // state: serde_json::from_str(&row.state).unwrap(),
                state: row.state,
            })
        })
}

#[server(SetMishState)]
async fn set_mish_state(name: String, state: String) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let state = serde_json::from_str(&state).unwrap();
    set_mish_state_query(&pool, &name, &state).await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn set_mish_state_query(
    pool: &sqlx::PgPool,
    name: &str,
    state: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let query = "
        INSERT INTO mish_states (name, state)
        VALUES ($1, $2::jsonb)
        ON CONFLICT (name) DO UPDATE SET
            state = EXCLUDED.state
    ";
    sqlx::query(query)
        .bind(name)
        .bind(state)
        .execute(pool)
        .await
        .map(|_| ())
}

#[component]
pub fn MishStatePage() -> impl IntoView {
    #[derive(Params, PartialEq)]
    struct MishStateParams {
        name: Option<String>,
    }
    let params = use_params::<MishStateParams>();
    let name = move || params.read().as_ref().unwrap().name.clone().unwrap();

    let set_mish_state_action = ServerAction::<SetMishState>::new();
    let values = Resource::new(
        move || (set_mish_state_action.version().get(), name()), // TODO add action value as input, and so we can use this value immediately instead of calling get_mish_state
        |(_version, name)| get_mish_state(name),
    );

    let toast = use_context::<ToastContext>().unwrap();
    Resource::new(
        move || {
            (
                set_mish_state_action.value().get(),
                set_mish_state_action.version().get(),
            )
        },
        move |value| async move {
            if matches!(value.0, Some(Ok(_))) {
                toast.set(Some(Toast("Mish State saved".to_owned())));
            }
        },
    );

    let raw_editor_mode = RwSignal::new(false);

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="mx-auto max-w-2xl space-y-16 sm:space-y-20 lg:mx-0 lg:max-w-none">
                <div>
                    <div>
                        <label for="raw-editor-mode">"RAW editor mode"</label>
                        <input type="checkbox" id="raw-editor-mode" bind:checked=raw_editor_mode />
                    </div>
                    <Suspense fallback=|| {
                        view! { <p>"Loading Mish State..."</p> }
                    }>
                        {move || {
                            values
                                .get()
                                .map(|values| {
                                    match values {
                                        Err(e) => {
                                            view! {
                                                <p>"Error loading Mish State: " {e.to_string()}</p>
                                            }
                                                .into_any()
                                        }
                                        Ok(value) => {
                                            value
                                                .map(|state| {
                                                    if raw_editor_mode.get() {
                                                        view! {
                                                            <RawEditor
                                                                name=state.name.clone()
                                                                state=state.state.to_string()
                                                                set_config_server_action=set_mish_state_action
                                                            />
                                                        }
                                                            .into_any()
                                                    } else {
                                                        match state.state {
                                                            serde_json::Value::Bool(b) => {
                                                                view! {
                                                                    <input
                                                                        type="checkbox"
                                                                        checked=b
                                                                        on:input:target=move |ev| {
                                                                            let value = ev.target().checked();
                                                                            set_mish_state_action
                                                                                .dispatch(SetMishState {
                                                                                    name: state.name.clone(),
                                                                                    state: serde_json::to_string(&value).unwrap(),
                                                                                });
                                                                        }
                                                                    />
                                                                }
                                                                    .into_any()
                                                            }
                                                            serde_json::Value::String(s) => {
                                                                view! {
                                                                    <TextEditor
                                                                        name=state.name.clone()
                                                                        state=s
                                                                        set_config_server_action=set_mish_state_action
                                                                    />
                                                                }
                                                                    .into_any()
                                                            }
                                                            _ => {
                                                                view! {
                                                                    <RawEditor
                                                                        name=state.name
                                                                        state=state.state.to_string()
                                                                        set_config_server_action=set_mish_state_action
                                                                    />
                                                                }
                                                                    .into_any()
                                                            }
                                                        }
                                                    }
                                                })
                                                .unwrap_or_else(|| {
                                                    view! {
                                                        <RawEditor
                                                            name=name()
                                                            state="".to_string()
                                                            set_config_server_action=set_mish_state_action
                                                        />
                                                    }
                                                        .into_any()
                                                })
                                        }
                                    }
                                })
                        }} <div>{move || format!("{:?}", values.get())}</div>
                    </Suspense>
                </div>
            </div>
        </main>
    }
}

// Dag inspector lists all mish_states, click on one to edit it
// Shows a nice JSON expansion of the state

// Keys:
// user.chris.fish_tank.light.blue
// user.chris.fish_tank.light.white
// user.chris.fish_tank.pump.filter

// = true // no encoding, no property value, etc.

// type Property = {
//   name: String,
//   schema: String, // IPLD schema?
// }
// type PropertyValue<T> = {
//   property: Property,
//   value: T,
// }
// blueLight = Property {
//   name: "Blue Light",
//   schema: "boolean"
// }
// whiteLight = Property {
//   name: "White Light",
//   schema: "boolean"
// }
// pump = Property {
//   name: "Filter Pump",
//   schema: "boolean"
// }
// blueLightState = PropertyValue {
//   property: blueLight,
//   value: true,
// }
// whiteLightState = PropertyValue {
//   property: whiteLight,
//   value: false,
// }
// pumpState = PropertyValue {
//   property: pump,
//   value: false,
// }
// user.chris.fish_tank = StateCapture [
//   blueLightState,
//   whiteLightState,
//   pumpState
// ]

// Alternative is this StateCapture is emitted/recorded as a change event in a blockchain. And then locally the state is updated to a StateCapture.

// UI
// When there are links, you can click on them to view the other DAG item
// You can edit the DAG item and save it
