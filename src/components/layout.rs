use {
    crate::{
        components::{
            mish::{
                dag_inspector_page::DagInspectorPage, ipld_blob_page::IpldBlobPage,
                mish_state_page::MishStatePage,
            },
            navbar::Navbar,
            pages::{
                actions_page::ActionsPage, configs_page::ConfigsPage,
                dashboard_page::DashboardPage, devices_page::DevicesPage,
                integrations_page::IntegrationsPage, login_page::LoginPage,
                settings_page::SettingsPage, websocket_page::WebSocketPage,
            },
        },
        error_template::{AppError, ErrorTemplate},
    },
    gloo_timers::callback::Timeout,
    leptos::prelude::*,
    leptos_meta::{Meta, Script, Stylesheet, Title, provide_meta_context},
    leptos_router::{
        components::{Route, Router, Routes},
        path,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast(pub String);

pub type ToastContext = RwSignal<Option<Toast>>;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    let toast = ToastContext::new(None);
    Effect::new(move |_| {
        if toast.get().is_some() {
            Timeout::new(500, move || {
                toast.set(None);
            })
        } else {
            Timeout::new(0, move || {})
        }
    });
    provide_context(toast);

    view! {
        <Stylesheet id="leptos" href="/pkg/iron_nest.css" />

        <Script src="https://cdn.tailwindcss.com" />
        <Script>
            "window.addEventListener('DOMContentLoaded', () => {
                const toggleButtons = document.querySelectorAll('.toggle-sidebar');
                const sidebar = document.querySelector('.sidebar');
                
                toggleButtons.forEach(button => {
                    button.addEventListener('click', () => {
                        console.log('Toggle sidebar clicked');
                        sidebar.classList.toggle('hidden');
                    });
                });
            });"
        </Script>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no" />
        <Meta name="apple-mobile-web-app-capable" content="yes" />
        <Meta name="mobile-web-app-capable" content="yes" />
        <Title text="IronNest" />

        {move || {
            toast
                .get()
                .map(|Toast(message)| {
                    view! {
                        <div class="fixed bottom-4 right-4 z-50">
                            <div class="bg-gray-900 text-white p-4 rounded-md shadow-md">
                                {message}
                            </div>
                        </div>
                    }
                })
        }}

        <div>
            <div class="relative z-50 hidden sidebar" role="dialog" aria-modal="true">
                <div class="fixed inset-0 bg-gray-900/80"></div>

                <div class="fixed inset-0 flex">
                    <div class="relative mr-16 flex w-full max-w-xs flex-1">
                        <div class="absolute left-full top-0 flex w-16 justify-center pt-5">
                            <button type="button" class="-m-2.5 p-2.5 toggle-sidebar">
                                <span class="sr-only">Close sidebar</span>
                                <svg
                                    class="h-6 w-6 text-white"
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

                        <div class="flex grow flex-col gap-y-5 overflow-y-auto bg-gray-900 px-6 pb-2 ring-1 ring-white/10">
                            <div class="flex h-16 shrink-0 items-center">
                                <img class="h-8 w-auto" src="/icon.png" alt="Iron Nest" />
                            </div>
                            <nav class="flex flex-1 flex-col">
                                <ul role="list" class="-mx-2 flex-1 space-y-1">
                                    <li>
                                        <a
                                            href="/"
                                            class="nav-link text-gray-400 flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
                                        >
                                            <svg
                                                class="h-6 w-6 shrink-0"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="currentColor"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25"
                                                ></path>
                                            </svg>
                                            "Dashboard"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="/devices"
                                            class="nav-link text-gray-400 flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
                                        >
                                            <svg
                                                class="h-6 w-6 shrink-0"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="currentColor"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M21.75 17.25v-.228a4.5 4.5 0 00-.12-1.03l-2.268-9.64a3.375 3.375 0 00-3.285-2.602H7.923a3.375 3.375 0 00-3.285 2.602l-2.268 9.64a4.5 4.5 0 00-.12 1.03v.228m19.5 0a3 3 0 01-3 3H5.25a3 3 0 01-3-3m19.5 0a3 3 0 00-3-3H5.25a3 3 0 00-3 3m16.5 0h.008v.008h-.008v-.008zm-3 0h.008v.008h-.008v-.008z"
                                                ></path>
                                            </svg>
                                            "Devices"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="/accounts"
                                            class="nav-link text-gray-400 hover:text-white hover:bg-gray-800 group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
                                        >
                                            <svg
                                                class="h-6 w-6 shrink-0"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="currentColor"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z"
                                                ></path>
                                            </svg>
                                            "Accounts"
                                        </a>
                                    </li>
                                </ul>
                            </nav>
                        </div>
                    </div>
                </div>
            </div>

            <Router>
                <Navbar />

                <div class="sticky top-0 z-40 flex items-center gap-x-6 bg-gray-900 px-4 py-4 shadow-sm sm:px-6 lg:hidden">
                    <button
                        type="button"
                        class="-m-2.5 p-2.5 text-gray-400 lg:hidden toggle-sidebar"
                    >
                        <span class="sr-only">"Open sidebar"</span>
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
                                d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5"
                            ></path>
                        </svg>
                    </button>
                    <div class="flex-1 text-sm font-semibold leading-6 text-white">"Dashboard"</div>
                    <a href="#">
                        <span class="sr-only">"Icon"</span>
                        <img class="h-8 w-8 rounded-full bg-gray-800" src="/icon.png" alt="" />
                    </a>
                </div>
                <main class="lg:pl-20 bg-blue-100 min-h-screen">
                    <Routes fallback=|| {
                        let mut errors = Errors::default();
                        errors.insert_with_default_key(AppError::NotFound);
                        view! { <ErrorTemplate errors /> }.into_view()
                    }>
                        <Route path=path!("/") view=DashboardPage />
                        <Route path=path!("/integrations") view=IntegrationsPage />
                        <Route path=path!("/integrations/:integration") view=LoginPage />
                        <Route path=path!("/actions") view=ActionsPage />
                        <Route path=path!("/settings") view=SettingsPage />
                        <Route path=path!("/settings/configs") view=ConfigsPage />
                        <Route path=path!("/settings/dag-inspector") view=DagInspectorPage />
                        <Route
                            path=path!("/settings/dag-inspector/mish-state/:name")
                            view=MishStatePage
                        />
                        <Route
                            path=path!("/settings/dag-inspector/ipld-blob/:cid")
                            view=IpldBlobPage
                        />
                        <Route path=path!("/devices") view=DevicesPage />
                        <Route path=path!("/websocket") view=WebSocketPage />
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
