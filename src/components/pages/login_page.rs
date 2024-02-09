use {crate::components::login_form::LoginForm, leptos::*, leptos_router::*};

cfg_if::cfg_if! {
    if #[cfg(feature = "ssr")] {
        use std::sync::Arc;
    }
}

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
    let value: RwSignal<Option<Result<String, ServerFnError>>> = handle_login.value();
    let name = "ring".to_owned();
    let logo = "logo".to_owned();

    view! {
        <LoginForm name=name logo=logo form_value=value>
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
        </LoginForm>
    }
}
