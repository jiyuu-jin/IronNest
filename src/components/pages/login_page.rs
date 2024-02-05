use {leptos::*, leptos_router::*, std::sync::Arc};

#[component]
pub fn LoginPage() -> impl IntoView {
    let params = use_params_map();
    let integration =
        move || params.with(|params| params.get("integration").cloned().unwrap_or_default());

    match integration().as_str() {
        "ring" => view! { <RingLoginPage/> },
        _ => view! { <LoginPageNotFound/> },
    }
}

#[component]
pub fn LoginPageNotFound() -> impl IntoView {
    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            "Login Not Found"
        </div>
    }
}

#[server(HandleLogin)]
pub async fn handle_login(
    username: String,
    password: String,
    tfa: String,
) -> Result<String, ServerFnError> {
    use crate::integrations::ring::client::RingRestClient;
    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let result = ring_rest_client
        .request_auth_token(&username, &password, &tfa)
        .await;

    Ok(result)
}

#[component]
pub fn RingLoginPage() -> impl IntoView {
    let handle_login = create_server_action::<HandleLogin>();
    let value = handle_login.value();

    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <div class="sm:mx-auto sm:w-full sm:max-w-sm">
                <img
                    class="mx-auto h-20 w-auto"
                    src="https://cdn.shopify.com/s/files/1/2393/8647/files/31291831386201.jpg?v=1701174026"
                    alt="IronNest"
                />
                <h2 class="mt-10 text-center text-2xl font-bold leading-9 tracking-tight text-gray-900">
                    "Sign in to your account"
                </h2>
            </div>

            <div class="mt-10 sm:mx-auto sm:w-full sm:max-w-sm">
                <ActionForm action=handle_login class="space-y-6">
                    <div>
                        <label for="email" class="block text-sm font-medium leading-6 text-white">
                            "Email address"
                        </label>
                        <div class="mt-2">
                            <input
                                id="email"
                                name="username"
                                type="text"
                                placeholder="Username"
                                autocomplete="email"
                                required
                                class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            />
                        </div>
                    </div>

                    <div>
                        <div class="flex items-center justify-between">
                            <label
                                for="password"
                                class="block text-sm font-medium leading-6 text-white"
                            >
                                Password
                            </label>
                        </div>
                        <div class="mt-2">
                            <input
                                type="password"
                                name="password"
                                placeholder="Password"
                                autocomplete="current-password"
                                required
                                class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            />
                        </div>
                    </div>
                    <div>
                        <div class="flex items-center justify-between">
                            <label for="tfa" class="block text-sm font-medium leading-6 text-white">
                                Password
                            </label>
                        </div>
                        <div class="mt-2">
                            <input
                                type="password"
                                name="tfa"
                                placeholder="2FA code"
                                class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            />
                        </div>
                    </div>

                    <div>
                        <input
                            type="submit"
                            value="Login"
                            class="flex w-full justify-center rounded-md bg-indigo-500 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm hover:bg-indigo-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500"
                        />
                    </div>
                </ActionForm>
                <p>{value}</p>
            </div>
        </div>
    }
}
