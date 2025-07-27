use {
    crate::{
        components::{
            layout::{Toast, ToastContext},
            select::Select,
            text_input::TextInput,
        },
        integrations::iron_nest::types::RequiredAction,
        server::actions::{AddAction, DeleteAction, RunAction, get_actions},
    },
    leptos::prelude::*,
};

#[component]
pub fn ActionsPage() -> impl IntoView {
    let create_action_action = ServerAction::<AddAction>::new();
    let delete_action_action = ServerAction::<DeleteAction>::new();
    let run_action_action = ServerAction::<RunAction>::new();
    let actions = Resource::new(
        move || {
            (
                create_action_action.version().get(),
                delete_action_action.version().get(),
            )
        },
        |_| get_actions(),
    );

    let (show_create_action, set_show_create_action) = signal(false);

    let toast = use_context::<ToastContext>().unwrap();
    Resource::new(
        move || {
            (
                create_action_action.value().get(),
                create_action_action.version().get(),
            )
        },
        move |value| async move {
            if matches!(value.0, Some(Ok(_))) {
                toast.set(Some(Toast("Action created".to_owned())));
            }
        },
    );
    Resource::new(
        move || {
            (
                delete_action_action.value().get(),
                delete_action_action.version().get(),
            )
        },
        move |value| async move {
            if matches!(value.0, Some(Ok(_))) {
                toast.set(Some(Toast("Action deleted".to_owned())));
            }
        },
    );
    Resource::new(
        move || {
            (
                run_action_action.value().get(),
                run_action_action.version().get(),
            )
        },
        move |value| async move {
            if matches!(value.0, Some(Ok(_))) {
                toast.set(Some(Toast("Action ran".to_owned())));
            }
        },
    );

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
                <h1 class="text-lg">"Actions"</h1>
                <hr class="mb-2" />
                <Suspense fallback=move || {
                    view! { <p>"Loading actions..."</p> }
                }>
                    {move || {
                        actions
                            .get()
                            .map(|data| {
                                match data {
                                    Ok(data) => {
                                        if data.is_empty() {
                                            view! { <p>"No actions found"</p> }.into_any()
                                        } else {
                                            view! {
                                                <ul class="device-list space-y-2">
                                                    {data
                                                        .into_iter()
                                                        .map(|action| {
                                                            let RequiredAction {
                                                                name,
                                                                cron,
                                                                function_name,
                                                                function_args,
                                                            } = action.fields;
                                                            view! {
                                                                <li>
                                                                    {format!(
                                                                        "{name}: {cron} -> {function_name}({function_args})",
                                                                    )}
                                                                    <button on:click=move |_| {
                                                                        delete_action_action
                                                                            .dispatch(DeleteAction { id: action.id });
                                                                    }>"Delete"</button>
                                                                    <button on:click=move |_| {
                                                                        run_action_action.dispatch(RunAction { id: action.id });
                                                                    }>"Run"</button>
                                                                </li>
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}

                                                </ul>
                                                {move || {
                                                    delete_action_action
                                                        .value()
                                                        .get()
                                                        .and_then(|value| {
                                                            value
                                                                .map_err(|value| {
                                                                    view! {
                                                                        <div>
                                                                            <p>"Delete action error: " {value.to_string()}</p>
                                                                        </div>
                                                                    }
                                                                })
                                                                .err()
                                                        })
                                                }}
                                            }
                                                .into_any()
                                        }
                                    }
                                    Err(e) => {
                                        view! { <p>{format!("GetActions error: {e}")}</p> }
                                            .into_any()
                                    }
                                }
                            })
                    }}

                </Suspense>
                <button
                    class="mt-4 bg-indigo-600 text-white px-4 py-2 rounded"
                    on:click=move |_| set_show_create_action.set(true)
                >
                    "Create Action"
                </button>
                {move || {
                    show_create_action
                        .get()
                        .then(|| {
                            view! {
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
                                                    <ActionForm action=create_action_action>

                                                        <div class="flex h-full flex-col divide-y divide-gray-200 bg-white shadow-xl">
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
                                                                            <TextInput
                                                                                label="Event Cron".to_owned()
                                                                                name="cron".to_owned()
                                                                                placeholder="".to_owned()
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
                                                                                        name="function_name".to_string()
                                                                                        data=vec![
                                                                                            "tplink_set_light_brightness".to_owned(),
                                                                                            "tplink_set_dimmer_brightness".to_owned(),
                                                                                            "tplink_turn_light_on_off".to_owned(),
                                                                                            "tplink_turn_plug_on".to_owned(),
                                                                                            "tplink_turn_plug_off".to_owned(),
                                                                                            "handle_smart_light_toggle".to_owned(),
                                                                                            "stoplight_toggle".to_owned(),
                                                                                        ]
                                                                                    />

                                                                                </div>
                                                                            </fieldset>
                                                                            <TextInput
                                                                                label="Functino args".to_owned()
                                                                                name="function_args".to_owned()
                                                                                placeholder="".to_owned()
                                                                                input_type="text".to_owned()
                                                                            />
                                                                        </div>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                            <div class="flex flex-shrink-0 justify-end px-4 py-4">
                                                                <button
                                                                    type="button"
                                                                    class="rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50"
                                                                    on:click=move |_| set_show_create_action.set(false)
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
                                                            {move || {
                                                                create_action_action
                                                                    .value()
                                                                    .get()
                                                                    .and_then(|value| {
                                                                        value
                                                                            .map_err(|value| {
                                                                                view! {
                                                                                    <div>
                                                                                        <p>"Create action error: " {value.to_string()}</p>
                                                                                    </div>
                                                                                }
                                                                                    .into_view()
                                                                            })
                                                                            .err()
                                                                    })
                                                            }}

                                                        </div>
                                                    </ActionForm>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            }
                        })
                }}

            </div>
        </main>
    }
}
