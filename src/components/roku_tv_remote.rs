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
        <div class="col-span-3 h-[264px] rounded-lg shadow-lg">
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
                                        <div class="col-span-4 h-[264px] flex flex-col">
                                            <div class="bg-white rounded-lg transition-all duration-500 dark:border-slate-500 p-2 xl:p-6 flex flex-col h-full overflow-hidden">
                                                <div class="grid grid-cols-5 gap-1 rounded-lg h-full content-center">
                                                    {data
                                                        .roku_apps
                                                        .into_iter()
                                                        .map(|app| {
                                                            view! {
                                                                <div
                                                                    class="bg-white border-slate-500 border rounded-lg flex items-center justify-center cursor-pointer shadow-sm w-full h-full"
                                                                    on:click=move |_| {
                                                                        let id = app.id.to_string();
                                                                        spawn_local(async move {
                                                                            launch_roku_app(id).await.unwrap();
                                                                        });
                                                                    }
                                                                >

                                                                    <img
                                                                        class="rounded-lg max-h-full max-w-full object-contain"
                                                                        src=format!("data:image/png;base64, {}", app.icon)
                                                                    />
                                                                </div>
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}
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
