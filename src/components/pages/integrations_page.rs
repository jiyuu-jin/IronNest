use {crate::integrations::iron_nest::shared::INTEGRATIONS, leptos::*};

#[component]
pub fn AccountsPage() -> impl IntoView {
    let integration_views: Vec<_> = INTEGRATIONS
        .iter()
        .map(|data| {
            view! {
                <li class="overflow-hidden rounded-xl border border-gray-200">
                    <a href=format!("/integrations/{}", data.name)>
                        <div class="flex items-center gap-x-4 border-b border-gray-900/5 bg-gray-50 p-6">
                            <img
                                src=data.image.clone()
                                alt=data.name.clone()
                                class="h-12 w-12 flex-none rounded-lg bg-white object-cover ring-1 ring-gray-900/10"
                            />
                            <div class="text-sm font-medium leading-6 text-gray-900">
                                {data.name}
                            </div>
                            <div class="relative ml-auto">
                                <button
                                    type="button"
                                    class="-m-2.5 block p-2.5 text-gray-400 hover:text-gray-500"
                                    id="options-menu-0-button"
                                    aria-expanded="false"
                                    aria-haspopup="true"
                                >
                                    <span class="sr-only">Open options</span>
                                    <svg
                                        class="h-5 w-5"
                                        viewBox="0 0 20 20"
                                        fill="currentColor"
                                        aria-hidden="true"
                                    >
                                        <path d="M3 10a1.5 1.5 0 113 0 1.5 1.5 0 01-3 0zM8.5 10a1.5 1.5 0 113 0 1.5 1.5 0 01-3 0zM15.5 8.5a1.5 1.5 0 100 3 1.5 1.5 0 000-3z"></path>
                                    </svg>
                                </button>
                            </div>
                        </div>
                        <dl class="-my-3 divide-y divide-gray-100 px-6 py-4 text-sm leading-6">
                            <div class="flex justify-between gap-x-4 py-3">
                                <dt class="text-gray-500">"Last authenticated"</dt>
                                <dd class="text-gray-700">
                                    <time datetime="2024-1-2">"January 2, 2024"</time>
                                </dd>
                            </div>
                        </dl>
                    </a>
                </li>
            }
        })
        .collect();

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <ul role="list" class="grid grid-cols-1 gap-x-6 gap-y-8 lg:grid-cols-3 xl:gap-x-8">
                {integration_views}
            </ul>
        </main>
    }
}
