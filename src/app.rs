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
        <Meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no" />
        <Meta name="apple-mobile-web-app-capable" content="yes" />
        <Meta name="mobile-web-app-capable" content="yes" />

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
    view! {
        <h1>"Iron Nest is Running"</h1>
        <p><a href="/login">Login</a></p>
        <p><a href="/rest-api/ring" rel="external">"Ring"</a></p>
        <p><a href="/rest-api/roku" rel="external">"Roku"</a></p>
    }
}

#[server(HandleLogin)]
pub async fn handle_login(
    username: String,
    password: String,
    tfa: String,
) -> Result<String, ServerFnError> {
    use {crate::integrations::ring::RingRestClient, std::sync::Arc};

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
        <p><A href="/app">"App"</A></p>
    }
}

#[server(GetWsUrl)]
pub async fn get_ws_url() -> Result<String, ServerFnError> {
    use {crate::integrations::ring::RingRestClient, std::sync::Arc};

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
