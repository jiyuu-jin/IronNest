use {super::pages::dashboard_page::DashboardValues, leptos::*};

#[component]
pub fn PlannedMeals(
    dashboard_values: Resource<(), Result<DashboardValues, ServerFnError>>,
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
                                        <ul>
                                            {data
                                                .scheduled_meals
                                                .into_iter()
                                                .map(|meal| {
                                                    view! { <li class="text-white">{meal.recipie_name}</li> }
                                                })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    }
                                        .into_view()
                                }
                                Err(e) => {
                                    view! { <div>{format!("PlannedMeals error: {e}")}</div> }
                                        .into_view()
                                }
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}
