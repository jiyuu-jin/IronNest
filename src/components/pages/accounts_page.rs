use leptos::*;

pub struct Integration {
    name: String,
    image: String,
}

#[component]
pub fn AccountsPage() -> impl IntoView {
    let integrations = vec![Integration {
        name: "ring".to_string(),
        image: "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcTIr91eAi3NC85wLntkOtCVTHPrrmK3gbvHcLASAbbJiOlqX4dTxttliz8uDi8mDfcRTzI&usqp=CAU"
            .to_string(),
    },Integration{
        name: "eufy".to_string(),
        image: "data:image/png;base64,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".to_string(),
    }];

    let integration_views: Vec<_> = integrations
        .iter()
        .map(|data| {
            view! {
                <li class="overflow-hidden rounded-xl border border-gray-200">
                    <a href=format!("/accounts/{}", data.name.clone())>
                        <div class="flex items-center gap-x-4 border-b border-gray-900/5 bg-gray-50 p-6">
                            <img
                                src=data.image.clone()
                                alt=data.name.clone()
                                class="h-12 w-12 flex-none rounded-lg bg-white object-cover ring-1 ring-gray-900/10"
                            />
                            <div class="text-sm font-medium leading-6 text-gray-900">
                                {data.name.clone()}
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
