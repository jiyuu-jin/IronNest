use {
    crate::components::{login_form::LoginForm, text_input::TextInput},
    leptos::prelude::*,
    leptos_router::hooks::use_params_map,
};

#[component]
pub fn LoginPage() -> impl IntoView {
    let params = use_params_map();
    let integration = move || params.with(|params| params.get("integration").unwrap_or_default());

    match integration().as_str() {
        "ring" => view! { <RingLoginPage /> }.into_any(),
        _ => view! { <LoginPageNotFound /> }.into_any(),
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

#[server(HandleRingLogin)]
pub async fn handle_ring_login(
    username: String,
    password: String,
    tfa: String,
) -> Result<String, ServerFnError> {
    use {crate::integrations::ring::client::RingRestClient, std::sync::Arc};
    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let result = ring_rest_client
        .request_auth_token(&username, &password, &tfa)
        .await;

    Ok(result)
}

#[component]
pub fn RingLoginPage() -> impl IntoView {
    let handle_login = ServerAction::<HandleRingLogin>::new();
    let value: RwSignal<Option<Result<String, ServerFnError>>> = handle_login.value();
    let name = "ring".to_owned();
    let logo = "logo".to_owned();

    view! {
        <LoginForm name=name logo=logo form_value=value>
            <ActionForm action=handle_login>
                <div class="space-y-6">
                    <TextInput
                        label="Username".to_owned()
                        name="username".to_owned()
                        placeholder="username".to_owned()
                        input_type="text".to_owned()
                    />
                    <TextInput
                        label="Password".to_owned()
                        name="password".to_owned()
                        placeholder="password".to_owned()
                        input_type="password".to_owned()
                    />
                    <TextInput
                        label="2FA code".to_owned()
                        name="tfa".to_owned()
                        placeholder="2FA code".to_owned()
                        input_type="password".to_owned()
                    />
                    <div>
                        <input
                            type="submit"
                            value="Login"
                            class="flex w-full justify-center rounded-md bg-indigo-500 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm hover:bg-indigo-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500"
                        />
                    </div>
                </div>
            </ActionForm>
        </LoginForm>
    }.into_any()
}
