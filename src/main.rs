use {
    iron_nest::{handlers::roku_keypress_handler, integrations::ring::RingRestClient},
    log::{error, info},
};

cfg_if::cfg_if! {
    if #[cfg(feature = "ssr")] {
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
                handlers::{ring_handler, roku_handler},
            },
            leptos::{get_configuration, logging::log, provide_context, LeptosOptions},
            leptos_axum::{generate_route_list, handle_server_fns_with_context, LeptosRoutes},
            reqwest::header::HeaderMap,
            std::{ sync::Arc},
            sqlx::sqlite::SqlitePool,
        };

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
            let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

            // Create the `devices` table
            sqlx::query(
                "CREATE TABLE devices (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    ip TEXT NOT NULL,
                    state TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await.unwrap();

            simple_logger::init_with_level(log::Level::Debug).expect("couldn't initialize logging");

            let conf = get_configuration(None).await.unwrap();
            let leptos_options = conf.leptos_options;
            let addr = leptos_options.site_addr;
            let routes = generate_route_list(App);

            let ring_rest_client = Arc::new(iron_nest::integrations::ring::RingRestClient::new());

            let app_state = AppState {
                leptos_options,
                ring_rest_client: ring_rest_client.clone(),
            };

            let iron_nest_router = Router::new()
                .route("/dashboard", get(ring_handler))
                .route("/roku", get(roku_handler))
                .route("/roku/:device_id/keypress/:key", get(roku_keypress_handler))
                .with_state(ring_rest_client.clone());

            let app = Router::new()
                .route(
                    "/api/*fn_name",
                    get(server_fn_handler).post(server_fn_handler),
                )
                .leptos_routes_with_handler(routes, get(leptos_routes_handler))
                .nest("/api", iron_nest_router)
                .fallback(file_and_error_handler)
                .with_state(app_state);

            let ring_auth_refresh_job = tokio::task::spawn(async move {
                let six_hours = chrono::Duration::hours(6).to_std().unwrap();
                let mut interval = tokio::time::interval(six_hours);
                loop {
                    interval.tick().await;

                    info!("Refreshing Ring auth token");
                    ring_rest_client.refresh_auth_token().await;
                }
            });

            let http_server = {
                log::info!("listening on http://{}", &addr);
                axum::Server::bind(&addr).serve(app.into_make_service())
            };

            tokio::select! {
                e = http_server => error!("HTTP server exiting with error {e:?}"),
                e = ring_auth_refresh_job => error!("Ring auth refresh job exiting with error {e:?}"),
            }
        }
    }
}
