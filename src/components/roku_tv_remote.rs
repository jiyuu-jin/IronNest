use {super::pages::dashboard_page::DashboardValues, leptos::*};

#[server(LaunchRokuApp)]
pub async fn launch_roku_app(app_id: String) -> Result<(), ServerFnError> {
    use crate::integrations::roku::roku_launch_app;
    roku_launch_app("10.0.0.217", &app_id).await;
    Ok(())
}

#[component]
pub fn RokuTvRemote(
    dashboard_values: Resource<(), Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="grid lg:grid-cols-12 grid-col-1 my-4 gap-2 max-h-60 overflow-auto">
            <Suspense fallback=|| {
                view! { <div>"Loading"</div> }
            }>
                {move || match dashboard_values.get() {
                    Some(data) => {
                        match data {
                            Ok(data) => {
                                view! {
                                    <div class="col-span-6 rounded-lg bg-slate-900">
                                        <div class="rounded-lg shadow-md col-span-6 h-full">
                                            <div class="border-slate-100 h-full rounded-lg transition-all duration-500 dark:bg-slate-800 transition-all duration-500 dark:border-slate-500 p-6 xl:p-10">
                                                <div class="grid grid-cols-4 gap-2 rounded-lg">
                                                    {data
                                                        .roku_apps
                                                        .into_iter()
                                                        .map(|app| {
                                                            view! {
                                                                <div
                                                                    class="w-20 bg-white rounded-lg align-middle text-center cursor-pointer shadow-sm"
                                                                    on:click=move |_| {
                                                                        let id = app.id.to_string();
                                                                        spawn_local(async move {
                                                                            launch_roku_app(id).await.unwrap();
                                                                        });
                                                                    }
                                                                >

                                                                    <img
                                                                        class="rounded-lg"
                                                                        src=format!("data:image/png;base64, {}", app.icon)
                                                                    />
                                                                </div>
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }
                            Err(_) => {
                                view! { <div></div> }
                            }
                        }
                    }
                    None => {
                        view! { <div></div> }
                    }
                }}

            </Suspense>
        </div>
    }
}
