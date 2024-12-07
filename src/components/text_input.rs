use leptos::prelude::*;

#[component]
pub fn TextInput(
    label: String,
    name: String,
    placeholder: String,
    input_type: String,
) -> impl IntoView {
    view! {
        <div>
            <label for=name.clone() class="block text-sm font-medium leading-6 text-white">
                {label}
            </label>
            <div class="mt-2">
                <input
                    type=input_type
                    id=name.clone()
                    name=name
                    placeholder=placeholder
                    class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                />
            </div>
        </div>
    }
}
