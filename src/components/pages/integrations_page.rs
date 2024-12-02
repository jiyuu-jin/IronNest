use {
    crate::{
        components::checkbox::Checkbox,
        server::integrations_page::{get_integrations, toggle_integration},
    },
    leptos::*,
};

#[component]
pub fn IntegrationsPage() -> impl IntoView {
    let integrations = create_resource(|| (), |_| get_integrations());

    let toggle_action = create_action(|(id, enabled, name): &(i64, bool, String)| {
        let id = *id;
        let enabled = *enabled;
        let name = name.clone();
        async move { toggle_integration(id, enabled, name).await }
    });

    view! {
        <Suspense fallback=|| {
            view! { <p>Loading</p> }
        }>
            {move || {
                integrations
                    .get()
                    .map(|integration| {
                        integration
                            .map(|integrations| {
                                let integration_views: Vec<_> = integrations
                                    .into_iter()
                                    .map(|data| {
                                        view! {
                                            <li class="overflow-hidden rounded-xl border border-gray-200">
                                                <a href=format!("/integrations/{}", data.name.clone())>
                                                    <div class="flex items-center gap-x-4 border-b border-gray-900/5 bg-gray-50 p-6">
                                                        <img
                                                            src=data.image
                                                            alt=data.name.clone()
                                                            class="h-12 w-12 flex-none rounded-lg bg-white object-cover ring-1 ring-gray-900/10"
                                                        />
                                                        <div class="text-sm font-medium leading-6 text-gray-900">
                                                            {data.name.clone()}
                                                        </div>
                                                        <div class="relative ml-auto">
                                                            <Checkbox
                                                                value=data.enabled
                                                                on_click=None
                                                                on_click_fn=Some(
                                                                    Box::new(move || {
                                                                        toggle_action
                                                                            .dispatch((data.id, !data.enabled, data.name.clone()))
                                                                    }),
                                                                )
                                                            />

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
                                        <ul
                                            role="list"
                                            class="grid grid-cols-1 gap-x-6 gap-y-8 lg:grid-cols-3 xl:gap-x-8"
                                        >
                                            {integration_views}
                                        </ul>
                                    </main>
                                }
                                    .into_view()
                            })
                            .unwrap_or_else(|e| {
                                view! { <p>"Error:" {e.to_string()}</p> }.into_view()
                            })
                    })
            }}

        </Suspense>
    }
}
