use leptos::*;

#[component]
pub fn Select(label: String, name: String, data: Vec<String>) -> impl IntoView {
    view! {
      <div>
        <label
            for="function-name"
            class="block text-sm font-medium leading-6 text-gray-900"
        >
          {label}
        </label>
        <div class="inline">
            <div class="mt-2 inline">
                <select
                    id={name.clone()}
                    name={name}
                    class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:max-w-xs sm:text-sm sm:leading-6"
                >
                {data.iter().map(|value| {
                    view! {
                      <option>{value}</option>
                    }
                  }).collect::<Vec<_>>()
                }
                </select>
            </div>
        </div>
      </div>
    }
}
