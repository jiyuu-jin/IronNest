use leptos::prelude::*;

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="mx-auto max-w-2xl space-y-16 sm:space-y-20 lg:mx-0 lg:max-w-none">
                <div>
                    <h2 class="text-base font-semibold leading-7 text-gray-900">"User Profile"</h2>
                    <p class="mt-1 text-sm leading-6 text-gray-500">
                        "User info for authentication & identification."
                    </p>

                    <dl class="mt-6 space-y-6 divide-y divide-gray-100 border-t border-gray-200 text-sm leading-6">
                        <div class="pt-6 sm:flex">
                            <dt class="font-medium text-gray-900 sm:w-64 sm:flex-none sm:pr-6">
                                Full name
                            </dt>
                            <dd class="mt-1 flex justify-between gap-x-6 sm:mt-0 sm:flex-auto">
                                <div class="text-gray-900">"Iron Nest User"</div>
                                <button
                                    type="button"
                                    class="font-semibold text-indigo-600 hover:text-indigo-500"
                                >
                                    Update
                                </button>
                            </dd>
                        </div>
                        <div class="pt-6 sm:flex">
                            <dt class="font-medium text-gray-900 sm:w-64 sm:flex-none sm:pr-6">
                                "Username"
                            </dt>
                            <dd class="mt-1 flex justify-between gap-x-6 sm:mt-0 sm:flex-auto">
                                <div class="text-gray-900">"jiyuu-jin"</div>
                                <button
                                    type="button"
                                    class="font-semibold text-indigo-600 hover:text-indigo-500"
                                >
                                    Update
                                </button>
                            </dd>
                        </div>
                        <div class="pt-6 sm:flex">
                            <dt class="font-medium text-gray-900 sm:w-64 sm:flex-none sm:pr-6">
                                Password
                            </dt>
                            <dd class="mt-1 flex justify-between gap-x-6 sm:mt-0 sm:flex-auto">
                                <div class="text-gray-900">"********"</div>
                                <button
                                    type="button"
                                    class="font-semibold text-indigo-600 hover:text-indigo-500"
                                >
                                    Update
                                </button>
                            </dd>
                        </div>
                    </dl>
                </div>

                <div>
                    <h2 class="text-base font-semibold leading-7 text-gray-900">
                        "Language and dates"
                    </h2>
                    <p class="mt-1 text-sm leading-6 text-gray-500">
                        "Choose what language and date format to use."
                    </p>

                    <dl class="mt-6 space-y-6 divide-y divide-gray-100 border-t border-gray-200 text-sm leading-6">
                        <div class="pt-6 sm:flex">
                            <dt class="font-medium text-gray-900 sm:w-64 sm:flex-none sm:pr-6">
                                Language
                            </dt>
                            <dd class="mt-1 flex justify-between gap-x-6 sm:mt-0 sm:flex-auto">
                                <div class="text-gray-900">English</div>
                                <button
                                    type="button"
                                    class="font-semibold text-indigo-600 hover:text-indigo-500"
                                >
                                    Update
                                </button>
                            </dd>
                        </div>
                        <div class="pt-6 sm:flex">
                            <dt class="font-medium text-gray-900 sm:w-64 sm:flex-none sm:pr-6">
                                Date format
                            </dt>
                            <dd class="mt-1 flex justify-between gap-x-6 sm:mt-0 sm:flex-auto">
                                <div class="text-gray-900">"DD-MM-YYYY"</div>
                                <button
                                    type="button"
                                    class="font-semibold text-indigo-600 hover:text-indigo-500"
                                >
                                    Update
                                </button>
                            </dd>
                        </div>
                        <div class="flex pt-6">
                            <dt
                                class="flex-none pr-6 font-medium text-gray-900 sm:w-64"
                                id="timezone-option-label"
                            >
                                Automatic timezone
                            </dt>
                            <dd class="flex flex-auto items-center justify-end">
                                <button
                                    type="button"
                                    class="bg-gray-200 flex w-8 cursor-pointer rounded-full p-px ring-1 ring-inset ring-gray-900/5 transition-colors duration-200 ease-in-out focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                                    role="switch"
                                    aria-checked="true"
                                    aria-labelledby="timezone-option-label"
                                >
                                    <span
                                        aria-hidden="true"
                                        class="translate-x-0 h-4 w-4 transform rounded-full bg-white shadow-sm ring-1 ring-gray-900/5 transition duration-200 ease-in-out"
                                    ></span>
                                </button>
                            </dd>
                        </div>
                    </dl>
                </div>

                <div>
                    <h2 class="text-base font-semibold leading-7 text-gray-900">"Advanced"</h2>
                    <p class="mt-1 text-sm leading-6 text-gray-500">"Advanced settings."</p>
                    <a href="/settings/configs">"Config Import/Export"</a>
                </div>
            </div>
        </main>
    }
}
