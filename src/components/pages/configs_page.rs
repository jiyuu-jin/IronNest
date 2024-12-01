use {leptos::*, leptos_router::ActionForm};

#[cfg(feature = "ssr")]
use crate::integrations::iron_nest::types::config::Config;

#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow)]
struct Row {
    data: sqlx::types::Json<Config>,
}

#[server(GetConfig)]
async fn get_config() -> Result<String, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let config = get_config_query(&pool).await?;
    let config = if let Some(config) = config {
        config
    } else {
        let actions = crate::server::actions::get_actions_query(&pool).await?;
        Config { actions }
    };
    let config = serde_yaml::to_string(&config).unwrap();
    Ok(config)
}

#[cfg(feature = "ssr")]
pub async fn get_config_query(pool: &sqlx::PgPool) -> Result<Option<Config>, sqlx::Error> {
    let query = "
        SELECT data
        FROM config
    ";
    sqlx::query_as(query)
        .fetch_optional(pool)
        .await
        .map(|row| row.map(|row: Row| row.data.0))
}

#[server(SetConfig)]
async fn set_config(config: String) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let config = serde_yaml::from_str(&config)?;
    set_config_query(&pool, config).await.map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn set_config_query(pool: &sqlx::PgPool, config: Config) -> Result<(), sqlx::Error> {
    let query = "
        INSERT INTO config (id, data)
        VALUES (1, $1)
        ON CONFLICT (id) DO UPDATE SET
            data = EXCLUDED.data
    ";
    sqlx::query(query)
        .bind(sqlx::types::Json(config))
        .execute(pool)
        .await
        .map(|_| ())
}

#[component]
pub fn ConfigsPage() -> impl IntoView {
    let set_config_server_action = create_server_action::<SetConfig>();
    let config = create_resource(move || set_config_server_action.version(), |_| get_config());

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="mx-auto max-w-2xl space-y-16 sm:space-y-20 lg:mx-0 lg:max-w-none">
                <div>
                    <h2 class="text-base font-semibold leading-7 text-gray-900">"Configs"</h2>
                    <p class="mt-1 text-sm leading-6 text-gray-500">
                        "Import and export your configuration in YAML format."
                    </p>
                    <Suspense fallback=|| {
                        view! { <p>"Loading config..."</p> }
                    }>
                        {move || {
                            config
                                .get()
                                .map(|config| {
                                    match config {
                                        Err(e) => {
                                            view! { <p>"Error loading config: " {e.to_string()}</p> }
                                                .into_view()
                                        }
                                        Ok(config) => {
                                            view! {
                                                <ActionForm action=set_config_server_action>
                                                    <textarea name="config">{config}</textarea>
                                                    <div class="flex flex-shrink-0 justify-end px-4 py-4">
                                                        <button
                                                            type="button"
                                                            class="rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50"
                                                        >
                                                            // on:click=move |_| set_show_create_action.set(false)
                                                            Cancel
                                                        </button>
                                                        <button
                                                            type="submit"
                                                            class="ml-4 inline-flex justify-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                                                        >
                                                            Create
                                                        </button>
                                                    </div>
                                                    {move || {
                                                        set_config_server_action
                                                            .value()
                                                            .get()
                                                            .and_then(|value| {
                                                                value
                                                                    .map_err(|value| {
                                                                        view! {
                                                                            <div>
                                                                                <p>"Error: " {value.to_string()}</p>
                                                                            </div>
                                                                        }
                                                                            .into_view()
                                                                    })
                                                                    .err()
                                                            })
                                                    }}

                                                </ActionForm>
                                            }
                                                .into_view()
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