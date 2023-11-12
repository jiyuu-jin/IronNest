cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use {
    axum::{
        body::Body as AxumBody,
        extract::{FromRef, Path, RawQuery, State},
        response::{IntoResponse, Response},
        routing::{get,},
        Router,
    },
    dotenv::dotenv,
    http::Request,
    iron_nest::{
        app::App,
        fileserv::file_and_error_handler,
        handlers::{ring_auth_handler, ring_handler},
        utils::RingRestClient,
    },
    leptos::{get_configuration, logging::log, provide_context, LeptosOptions},
    leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes},
    reqwest::header::HeaderMap,
    std::{ sync::Arc},
};

/// This takes advantage of Axum's SubStates feature by deriving FromRef. This is the only way to have more than one
/// item in Axum's State. Leptos requires you to have leptosOptions in your State struct for the leptos route handlers
#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub ring_rest_client: Arc<RingRestClient>,
}

async fn server_fn_handler(
    State(app_state): State<AppState>,
    path: Path<String>,
    headers: HeaderMap,
    raw_query: RawQuery,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    log!("{:?}", path);

    handle_server_fns_with_context(
        path,
        headers,
        raw_query,
        move || {
            provide_context(app_state.ring_rest_client.clone());
        },
        request,
    )
    .await
}

async fn leptos_routes_handler(
    State(app_state): State<AppState>,
    req: Request<AxumBody>,
) -> Response {
    let handler = leptos_axum::render_app_to_stream_with_context(
        app_state.leptos_options.clone(),
        move || {
            provide_context(app_state.ring_rest_client.clone());
        },
        App,
    );
    handler(req).await.into_response()
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let ring_rest_client = Arc::new(iron_nest::utils::RingRestClient::new());

    let app_state = AppState {
        leptos_options,
        ring_rest_client: ring_rest_client.clone(),
    };

    let iron_nest_router = Router::new()
        .route("/ring", get(ring_handler))
        .route("/ring/auth", get(ring_auth_handler))
        .with_state(ring_rest_client);

    let app = Router::new()
        .route(
            "/api/*fn_name",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .nest("/rest-api", iron_nest_router)
        .fallback(file_and_error_handler)
        .with_state(app_state);

    log::info!("listening on http://{}", &addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

}}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
