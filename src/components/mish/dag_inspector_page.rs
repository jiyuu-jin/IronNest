use {
    crate::components::mish::new_mish_state_dialog::NewMishStateDialog,
    leptos::{
        prelude::*,
        server::{Resource, ServerAction},
    },
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

#[server(GetMishStates)]
async fn get_mish_states() -> Result<Vec<MishState>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let mish_states = get_mish_states_query(&pool).await?;
    Ok(mish_states)
}

#[cfg(feature = "ssr")]
pub async fn get_mish_states_query(pool: &sqlx::PgPool) -> Result<Vec<MishState>, sqlx::Error> {
    let query = "
        SELECT name, state
        FROM mish_states
    ";
    sqlx::query_as::<_, MishStateRow>(query)
        .fetch_all(pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| MishState {
                    name: row.name,
                    state: row.state,
                })
                .collect()
        })
}

#[component]
pub fn DagInspectorPage() -> impl IntoView {
    let set_config_server_action = ServerAction::<GetMishStates>::new();
    let values = Resource::new(
        move || set_config_server_action.version(),
        |_| get_mish_states(),
    );

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="mx-auto max-w-2xl space-y-16 sm:space-y-20 lg:mx-0 lg:max-w-none">
                <div>
                    <h2 class="text-base font-semibold leading-7 text-gray-900">"Configs"</h2>
                    <p class="mt-1 text-sm leading-6 text-gray-500">
                        "Import and export your configuration in YAML format."
                    </p>
                    <NewMishStateDialog />
                    <Suspense fallback=|| {
                        view! { <p>"Loading Mish States..."</p> }
                    }>
                        {move || {
                            values
                                .get()
                                .map(|values| {
                                    match values {
                                        Err(e) => {
                                            view! { <p>"Error loading values: " {e.to_string()}</p> }
                                                .into_any()
                                        }
                                        Ok(values) => {
                                            view! {
                                                {values
                                                    .into_iter()
                                                    .map(|state| {
                                                        view! {
                                                            <div>
                                                                <h3>
                                                                    <a href=format!(
                                                                        "/settings/dag-inspector/mish-state/{}",
                                                                        state.name,
                                                                    )>{state.name.clone()}</a>
                                                                </h3>
                                                            </div>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            }
                                                .into_any()
                                        }
                                    }
                                })
                        }}
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
