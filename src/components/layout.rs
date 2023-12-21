use {
    crate::{
        components::{
            navbar::Navbar,
            pages::{
                dashboard_page::DashboardPage, login_page::LoginPage, websocket_page::WebSocketPage,
            },
        },
        error_template::{AppError, ErrorTemplate},
    },
    leptos::*,
    leptos_meta::*,
    leptos_router::*,
};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Script src="https://cdn.tailwindcss.com"/>
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
        <Meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no"/>
        <Meta name="apple-mobile-web-app-capable" content="yes"/>
        <Meta name="mobile-web-app-capable" content="yes"/>
        <div>
            <div class="relative z-50 lg:hidden sidebar" role="dialog" aria-modal="true">
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
                                <img class="h-8 w-auto" src="/icon.png" alt="Iron Nest"/>
                            </div>
                            <nav class="flex flex-1 flex-col">
                                <ul role="list" class="-mx-2 flex-1 space-y-1">
                                    <li>
                                        <a
                                            href="/"
                                            class="bg-gray-800 text-white group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
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
                                            Dashboard
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="#"
                                            class="text-gray-400 hover:text-white hover:bg-gray-800 group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
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
                                            Accounts
                                        </a>
                                    </li>
                                </ul>
                            </nav>
                        </div>
                    </div>
                </div>
            </div>

            <Navbar/>

            <div class="sticky top-0 z-40 flex items-center gap-x-6 bg-gray-900 px-4 py-4 shadow-sm sm:px-6 lg:hidden">
                <button type="button" class="-m-2.5 p-2.5 text-gray-400 lg:hidden toggle-sidebar">
                    <span class="sr-only">Open sidebar</span>
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
                <div class="flex-1 text-sm font-semibold leading-6 text-white">Dashboard</div>
                <a href="#">
                    <span class="sr-only">Your profile</span>
                    <img class="h-8 w-8 rounded-full bg-gray-800" src="/icon.png" alt=""/>
                </a>
            </div>

            <Router fallback=|| {
                let mut outside_errors = Errors::default();
                outside_errors.insert_with_default_key(AppError::NotFound);
                view! { <ErrorTemplate outside_errors/> }.into_view()
            }>
                <main>
                    <Routes>
                        <Route path="/login" view=LoginPage/>
                        <Route path="/" view=DashboardPage/>
                        <Route path="/websocket" view=WebSocketPage/>
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
