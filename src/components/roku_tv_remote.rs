use {
    super::pages::dashboard_page::DashboardValues,
    leptos::{prelude::*, task::spawn_local},
};

#[server(LaunchRokuApp)]
pub async fn launch_roku_app(app_id: String) -> Result<(), ServerFnError> {
    use crate::integrations::roku::roku_launch_app;
    roku_launch_app("10.0.0.217", &app_id).await;
    Ok(())
}

#[component]
pub fn RokuTvRemote(
    dashboard_values: Resource<Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="col-span-6 rounded-lg bg-slate-900">
            <Suspense fallback=|| {
                view! { <div>"Loading"</div> }
            }>
                {move || {
                    dashboard_values
                        .get()
                        .map(|data| {
                            match data {
                                Ok(data) => {
                                    view! {
                                        <div>
                                            <div class="rounded-lg shadow-md col-span-4 resize">
                                                <div class="bg-white h-full rounded-lg transition-all duration-500 transition-all duration-500 dark:border-slate-500 p-2 xl:p-6">
                                                    <div class="grid grid-cols-5 gap-1 rounded-lg">
                                                        {data
                                                            .roku_apps
                                                            .into_iter()
                                                            .map(|app| {
                                                                view! {
                                                                    <div
                                                                        class="w-20 bg-white border-slate-500 border rounded-lg align-middle text-center cursor-pointer shadow-sm"
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
                                        .into_any()
                                }
                                Err(e) => {
                                    view! { <div>{format!("RokuTvRemote error: {e}")}</div> }
                                        .into_any()
                                }
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}
