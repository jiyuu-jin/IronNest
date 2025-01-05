use {
    super::pages::dashboard_page::DashboardValues,
    crate::integrations::instacart::types::ScheduledMeal, leptos::prelude::*,
};

type PlannedMealsData = Vec<(String, Vec<(ScheduledMeal, ScheduledMeal, ScheduledMeal)>)>;

pub fn group_meals(scheduled_meals: Vec<ScheduledMeal>) -> PlannedMealsData {
    scheduled_meals
        .chunks(3)
        .enumerate()
        .filter_map(|(index, chunk)| {
            let day = match index {
                0 => "Today".to_owned(),
                1 => "Tomorrow".to_owned(),
                _ => format!("Day {}", index + 1),
            };

            if chunk.len() < 3 {
                return None;
            }

            let day_meals = (chunk[0].clone(), chunk[1].clone(), chunk[2].clone());
            Some((day, vec![day_meals]))
        })
        .collect()
}

#[component]
pub fn PlannedMeals(
    dashboard_values: Resource<Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    view! {
        <Suspense fallback=|| {
            view! { <div>"Loading"</div> }
        }>
            {move || {
                dashboard_values
                    .get()
                    .map(|data| {
                        match data {
                            Ok(data) => {
                                let meal_plans = group_meals(data.scheduled_meals);
                                view! {
                                    <div class="col-span-3 h-[264px] rounded-lg bg-white text-black shadow-md p-4 flex flex-col shadow-lg">
                                        <div class="text-xl font-bold text-center mb-2">Meals</div>
                                        <div class="flex-1 grid grid-cols-2 gap-2 h-full">
                                            {meal_plans
                                                .into_iter()
                                                .map(|(day, meals)| {
                                                    view! {
                                                        <div class="bg-gray-100 p-2 rounded-lg flex flex-col justify-between h-full">
                                                            <div class="text-sm font-semibold text-center mb-1">
                                                                {day}
                                                            </div>
                                                            <div class="text-xs flex flex-col gap-2">
                                                                {meals
                                                                    .clone()
                                                                    .into_iter()
                                                                    .map(|(breakfast, lunch, dinner)| {
                                                                        view! {
                                                                            <div>
                                                                                <p class="font-bold">"Breakfast:"</p>
                                                                                <p>{breakfast.recipie_name.clone()}</p>
                                                                            </div>
                                                                            <div>
                                                                                <p class="font-bold">"Lunch:"</p>
                                                                                <p>{lunch.recipie_name.clone()}</p>
                                                                            </div>
                                                                            <div>
                                                                                <p class="font-bold">"Dinner:"</p>
                                                                                <p>{dinner.recipie_name.clone()}</p>
                                                                            </div>
                                                                        }
                                                                    })
                                                                    .collect::<Vec<_>>()}
                                                            </div>
                                                        </div>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <div>{format!("PlannedMeals error: {e}")}</div> }.into_any()
                            }
                        }
                    })
            }}

        </Suspense>
    }
}
