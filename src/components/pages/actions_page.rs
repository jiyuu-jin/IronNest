use {
    crate::components::{select::Select, text_input::TextInput},
    leptos::*,
};

// #[server(HandleCreateAction)]
// pub async fn handle_ring_login(
//     username: String,
//     password: String,
//     tfa: String,
// ) -> Result<String, ServerFnError> {
//     use crate::integrations::iron_nest::execute_function;
//     let function_name = tool_call.function.name.to_string();
//     let function_args: serde_json::Value = tool_call.function.arguments.parse().unwrap();
//     let function_response = execute_function(function_name, function_args).await;

//     Ok(result)
// }

#[component]
pub fn ActionsPage() -> impl IntoView {
    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
                <h1>"No Actions"</h1>
                <div
                    class="relative z-10"
                    aria-labelledby="slide-over-title"
                    role="dialog"
                    aria-modal="true"
                >
                    <div class="fixed inset-0"></div>

                    <div class="fixed inset-0 overflow-hidden">
                        <div class="absolute inset-0 overflow-hidden">
                            <div class="pointer-events-none fixed inset-y-0 right-0 flex max-w-full pl-10 sm:pl-16">
                                <div class="pointer-events-auto w-screen max-w-md">
                                    <form class="flex h-full flex-col divide-y divide-gray-200 bg-white shadow-xl">
                                        <div class="h-0 flex-1 overflow-y-auto">
                                            <div class="bg-indigo-700 px-4 py-6 sm:px-6">
                                                <div class="flex items-center justify-between">
                                                    <h2
                                                        class="text-base font-semibold leading-6 text-white"
                                                        id="slide-over-title"
                                                    >
                                                        "Schedule an Action"
                                                    </h2>
                                                    <div class="ml-3 flex h-7 items-center">
                                                        <button
                                                            type="button"
                                                            class="relative rounded-md bg-indigo-700 text-indigo-200 hover:text-white focus:outline-none focus:ring-2 focus:ring-white"
                                                        >
                                                            <span class="absolute -inset-2.5"></span>
                                                            <span class="sr-only">Close panel</span>
                                                            <svg
                                                                class="h-6 w-6"
                                                                fill="none"
                                                                viewBox="0 0 24 24"
                                                                stroke-width="1.5"
                                                                stroke="currentColor"
                                                                aria-hidden="true"
                                                            >
                                                                <path
                                                                    stroke-linecap="round"
                                                                    stroke-linejoin="round"
                                                                    d="M6 18L18 6M6 6l12 12"
                                                                ></path>
                                                            </svg>
                                                        </button>
                                                    </div>
                                                </div>
                                                <div class="mt-1">
                                                    <p class="text-sm text-indigo-300">
                                                        "Create a new one time or recurring event"
                                                    </p>
                                                </div>
                                            </div>
                                            <div class="flex flex-1 flex-col justify-between">
                                                <div class="divide-y divide-gray-200 px-4 sm:px-6">
                                                    <div class="space-y-6 pb-5 pt-6">
                                                        <TextInput
                                                            label="Event Name".to_owned()
                                                            name="name".to_owned()
                                                            placeholder="New event".to_owned()
                                                            input_type="text".to_owned()
                                                        />
                                                        <fieldset>
                                                            <legend class="text-sm font-medium leading-6 text-gray-900">
                                                                Repeat
                                                            </legend>
                                                            <div class="mt-2 space-y-4">
                                                                <div class="relative flex items-start">
                                                                    <div class="absolute flex h-6 items-center">
                                                                        <input
                                                                            id="privacy-public"
                                                                            name="privacy"
                                                                            aria-describedby="privacy-public-description"
                                                                            type="radio"
                                                                            class="h-4 w-4 border-gray-300 text-indigo-600 focus:ring-indigo-600"
                                                                            checked
                                                                        />
                                                                    </div>
                                                                    <div class="pl-7 text-sm leading-6">
                                                                        <label
                                                                            for="privacy-public"
                                                                            class="font-medium text-gray-900"
                                                                        >
                                                                            "Repeat"
                                                                            <div class="inline">
                                                                                <div class="mt-2 inline">
                                                                                    <select
                                                                                        id="repeats"
                                                                                        name="repeats"
                                                                                        autocomplete="repeats"
                                                                                        class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:max-w-xs sm:text-sm sm:leading-6"
                                                                                    >
                                                                                        <option>"Hourly"</option>
                                                                                        <option>"Daily"</option>
                                                                                        <option>"Weekly"</option>
                                                                                    </select>
                                                                                </div>
                                                                            </div>
                                                                        </label>
                                                                    </div>
                                                                </div>
                                                                <div>
                                                                    <div class="relative flex items-start">
                                                                        <div class="absolute flex h-6 items-center">
                                                                            <input
                                                                                id="privacy-private-to-project"
                                                                                name="privacy"
                                                                                aria-describedby="privacy-private-to-project-description"
                                                                                type="radio"
                                                                                class="h-4 w-4 border-gray-300 text-indigo-600 focus:ring-indigo-600"
                                                                            />
                                                                        </div>
                                                                        <div class="pl-7 text-sm leading-6">
                                                                            <label
                                                                                for="privacy-private-to-project"
                                                                                class="font-medium text-gray-900"
                                                                            >
                                                                                "One time"
                                                                            </label>
                                                                            <p
                                                                                id="privacy-private-to-project-description"
                                                                                class="text-gray-500"
                                                                            >
                                                                                "Will only run one time."
                                                                            </p>
                                                                        </div>
                                                                    </div>
                                                                </div>
                                                                <Select
                                                                    label="Function name".to_string()
                                                                    name="function-name".to_string()
                                                                    data=vec![
                                                                        "tplink_set_light_brightness".to_owned(),
                                                                        "tplink_turn_light_on_off".to_owned(),
                                                                        "tplink_turn_plug_on".to_owned(),
                                                                        "tplink_turn_plug_off".to_owned(),
                                                                        "stoplight_toggle".to_owned(),
                                                                    ]
                                                                />

                                                            </div>
                                                        </fieldset>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                        <div class="flex flex-shrink-0 justify-end px-4 py-4">
                                            <button
                                                type="button"
                                                class="rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50"
                                            >
                                                Cancel
                                            </button>
                                            <button
                                                type="submit"
                                                class="ml-4 inline-flex justify-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                                            >
                                                Create
                                            </button>
                                        </div>
                                    </form>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </main>
    }
}
