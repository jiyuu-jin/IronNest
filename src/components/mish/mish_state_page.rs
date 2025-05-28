use {
    crate::components::{
        layout::{Toast, ToastContext},
        mish::{editor::Editor, json_editor::JsonEditor},
    },
    ipld_core::codec::Links,
    leptos::prelude::*,
    leptos_router::{
        hooks::{use_navigate, use_params},
        params::Params,
    },
    serde::{Deserialize, Serialize},
    serde_ipld_dagjson::codec::DagJsonCodec,
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
    use crate::integrations::iron_nest::mish::MishStateModification;
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let mish_state_modification_bus_sender =
        use_context::<tokio::sync::mpsc::UnboundedSender<MishStateModification>>().unwrap();
    let state = hex::decode(state).unwrap();
    let state = serde_json::from_slice(&state).unwrap();
    set_mish_state_query(&pool, &name, &state).await?;
    mish_state_modification_bus_sender
        .send(MishStateModification::CreateOrUpdate { name, state })
        .unwrap();
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

#[server(DeleteMishState)]
async fn delete_mish_state(name: String) -> Result<(), ServerFnError> {
    use crate::integrations::iron_nest::mish::MishStateModification;
    let pool = use_context::<sqlx::PgPool>().expect("PgPool context should be set");
    let mish_state_modification_bus_sender =
        use_context::<tokio::sync::mpsc::UnboundedSender<MishStateModification>>().unwrap();
    delete_mish_state_query(&pool, &name).await?;
    mish_state_modification_bus_sender
        .send(MishStateModification::Delete { name })
        .unwrap();
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn delete_mish_state_query(pool: &sqlx::PgPool, name: &str) -> Result<(), sqlx::Error> {
    let query = "
        DELETE FROM mish_states
        WHERE name = $1
    ";
    sqlx::query(query)
        .bind(name)
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

    let set_mish_state_action2 = move |state: Vec<u8>| {
        set_mish_state_action.dispatch(SetMishState {
            name: name(),
            state: hex::encode(state),
        });
    };

    let delete_mish_state_action = ServerAction::<DeleteMishState>::new();

    let toast = use_context::<ToastContext>().unwrap();
    Resource::new(
        move || {
            (
                set_mish_state_action.value().get(),
                set_mish_state_action.version().get(),
            )
        },
        move |(value, _version)| async move {
            if matches!(value, Some(Ok(_))) {
                toast.set(Some(Toast("Mish State saved".to_owned())));
            }
        },
    );

    Resource::new(
        move || {
            (
                delete_mish_state_action.value().get(),
                delete_mish_state_action.version().get(),
            )
        },
        move |(value, _version)| async move {
            if matches!(value, Some(Ok(_))) {
                toast.set(Some(Toast("Mish State deleted".to_owned())));
                let navigate = use_navigate();
                navigate("/settings/dag-inspector", Default::default());
            }
        },
    );

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="mx-auto max-w-2xl space-y-16 sm:space-y-20 lg:mx-0 lg:max-w-none">
                <div>
                    <div>
                        <a href="/settings/dag-inspector">"Back to Dag Inspector"</a>
                    </div>
                    <Suspense fallback=|| {
                        view! { <p>"Loading Mish State..."</p> }
                    }>
                        <div>
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
                                                        view! {
                                                            <Editor state=state.state action=set_mish_state_action2 />
                                                        }
                                                            .into_any()
                                                    })
                                                    .unwrap_or_else(|| {
                                                        view! {
                                                            <JsonEditor
                                                                state=None
                                                                set_config_server_action=set_mish_state_action2
                                                            />
                                                        }
                                                            .into_any()
                                                    })
                                            }
                                        }
                                    })
                            }}
                        </div>
                        <div>{move || format!("{:?}", values.get())}</div>
                        <div>
                            {move || {
                                if let Some(Ok(Some(value))) = values.get() {
                                    let state = serde_json::to_vec(&value.state).unwrap();
                                    let links = <DagJsonCodec as Links>::links(&state);
                                    match links {
                                        Ok(links) => {
                                            view! {
                                                {links
                                                    .into_iter()
                                                    .map(|link| {
                                                        view! {
                                                            <p>
                                                                <a href=format!(
                                                                    "/settings/dag-inspector/ipld-blob/{link}",
                                                                )>"Link: "{link.to_string()}</a>
                                                            </p>
                                                        }
                                                            .into_any()
                                                    })
                                                    .collect::<Vec<_>>()}
                                            }
                                                .into_any()
                                        }
                                        Err(e) => {
                                            view! { <p>{format!("Error getting links: {e}")}</p> }
                                                .into_any()
                                        }
                                    }
                                } else {
                                    ().into_any()
                                }
                            }}
                        </div>
                        <button on:click=move |_| {
                            delete_mish_state_action.dispatch(DeleteMishState { name: name() });
                        }>"Delete"</button>
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
