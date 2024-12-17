use {super::pages::dashboard_page::DashboardValues, leptos::prelude::*};

#[component]
pub fn PlannedMeals(
    dashboard_values: Resource<Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    let meals = vec![
        (
            "Today",
            "Pancakes & Eggs",
            "Grilled Chicken Salad",
            "Spaghetti Bolognese",
        ),
        (
            "Tomorrow",
            "French Toast & Sausage",
            "Turkey Club Sandwich",
            "Grilled Salmon with Quinoa",
        ),
    ];

    view! {
        <div class="col-span-3 h-[264px] rounded-lg bg-white text-black shadow-md p-4 flex flex-col shadow-lg">
            <div class="text-xl font-bold text-center mb-2">Meals</div>
            <div class="flex-1 grid grid-cols-2 gap-2 h-full">
                {meals
                    .into_iter()
                    .map(|(day, breakfast, lunch, dinner)| {
                        view! {
                            <div class="bg-gray-100 p-2 rounded-lg flex flex-col justify-between h-full">
                                <div class="text-sm font-semibold text-center mb-1">{day}</div>
                                <div class="text-xs flex flex-col gap-2">
                                    <div>
                                        <p class="font-bold">Breakfast:</p>
                                        <p>{breakfast}</p>
                                    </div>
                                    <div>
                                        <p class="font-bold">Lunch:</p>
                                        <p>{lunch}</p>
                                    </div>
                                    <div>
                                        <p class="font-bold">Dinner:</p>
                                        <p>{dinner}</p>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
        </div>
    }
}
