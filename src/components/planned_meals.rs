use {super::pages::dashboard_page::DashboardValues, leptos::prelude::*};

#[component]
pub fn PlannedMeals(
    dashboard_values: Resource<Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div class="col-span-6 rounded-lg bg-white text-black">
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
                                                    view! { <li>{meal.recipie_name}</li> }
                                                })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    }
                                        .into_any()
                                }
                                Err(e) => {
                                    view! { <div>{format!("PlannedMeals error: {e}")}</div> }
                                        .into_any()
                                }
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}
