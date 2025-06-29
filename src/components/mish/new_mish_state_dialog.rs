use leptos::prelude::*;

#[component]
pub fn NewMishStateDialog() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (is_open, set_is_open) = signal(false);

    view! {
        <div>
            <button
                on:click=move |_| set_is_open.set(true)
                class="mt-2 rounded-md bg-indigo-600 px-3.5 py-2.5 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
            >
                "New Mish State"
            </button>

            {move || {
                is_open
                    .get()
                    .then(|| {
                        view! {
                            <div class="fixed inset-0 bg-gray-500 bg-opacity-75 flex items-center justify-center">
                                <div class="bg-white p-6 rounded-lg shadow-xl max-w-md w-full">
                                    <h3 class="text-lg font-medium leading-6 text-gray-900 mb-4">
                                        "Create New Mish State"
                                    </h3>
                                    <div class="mb-4">
                                        <label
                                            for="name"
                                            class="block text-sm font-medium text-gray-700"
                                        >
                                            "Name"
                                        </label>
                                        <input
                                            type="text"
                                            id="name"
                                            name="name"
                                            class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                                            prop:value=name
                                            on:input=move |ev| {
                                                set_name.set(event_target_value(&ev));
                                            }
                                        />
                                    </div>
                                    <div class="flex justify-end space-x-3">
                                        <button
                                            type="button"
                                            on:click=move |_| {
                                                set_is_open.set(false);
                                            }
                                            class="rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50"
                                        >
                                            "Cancel"
                                        </button>
                                        <a
                                            href=move || {
                                                format!("/settings/dag-inspector/mish-state/{}", name.get())
                                            }
                                            class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                                        >
                                            "Create"
                                        </a>
                                    </div>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}
