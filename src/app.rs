use {
    crate::error_template::{AppError, ErrorTemplate},
    leptos::*,
    leptos_meta::*,
    leptos_router::*,
};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/{{project-name}}.css"/>

        // sets the document title
        <Title text="Welcome to Iron Nest"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="/login" view=LoginPage/>
                    <Route path="/app" view=AppPage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! {
        <h1>"Welcome to Iron Nest!"</h1>
        <p><button on:click=on_click>"Click Me: " {count}</button></p>
        <p><a href="/login">Login</a></p>
    }
}

#[server(HandleLogin)]
pub async fn handle_login(
    username: String,
    password: String,
    tfa: String,
) -> Result<String, ServerFnError> {
    use {crate::utils::RingRestClient, std::sync::Arc};

    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let result = ring_rest_client
        .request_auth_token(&username, &password, &tfa)
        .await;

    Ok(result)
}

#[component]
fn LoginPage() -> impl IntoView {
    let handle_login = create_server_action::<HandleLogin>();
    let value = handle_login.value();

    view! {
        <h1>"Login"</h1>
        <ActionForm action=handle_login>
            <input type="text" name="username" placeholder="Username"/>
            <input type="password" name="password" placeholder="Password"/>
            <input type="password" name="tfa" placeholder="2FA code"/>
            <input type="submit" value="Login"/>
        </ActionForm>
        <p>{value}</p>
        <p><a href="/app">"App"</a></p>
        <p><a href="/rest-api/ring">"HTTP Dashboard"</a></p>
    }
}

#[server(GetWsUrl)]
pub async fn get_ws_url() -> Result<String, ServerFnError> {
    use {crate::utils::RingRestClient, std::sync::Arc};

    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let result = ring_rest_client.get_ws_url().await;

    Ok(result)
}

#[component]
fn AppPage() -> impl IntoView {
    let ws_url = create_resource(|| (), |_| get_ws_url());

    view! {
        <h1>"Dashboard"</h1>
        <Suspense
            fallback=move || view! { <p>"Loading..."</p> }>
            {move || ws_url.get().map(|data| view! { <p>{data}</p> })}
        </Suspense>
    }
}
