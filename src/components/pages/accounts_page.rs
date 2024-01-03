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
    }, Integration{
        name: "alexa".to_string(),
        image: "https://upload.wikimedia.org/wikipedia/commons/c/cc/Amazon_Alexa_App_Logo.png".to_string(),
    }, Integration{
        name: "tuya".to_string(),
        image: "data:image/png;base64,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".to_string(),
    }, Integration{
        name: "openai".to_string(),
        image:"https://play-lh.googleusercontent.com/6qi3w4uqKaD1c-CBdkkfO6IL0lH4OoCTEdiX0oYbLFxwfvxu1t8vuwHcagdYSFmFKmI=w480-h960-rw".to_string(),
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
