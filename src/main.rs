use {
    iron_nest::{
        handlers::roku_keypress_handler,
        integrations::{
            iron_nest::{client::insert_devices_into_db, types::Device},
            ring::RingRestClient,
            roku::roku_discover,
            tplink::discover_devices,
        },
    },
    log::{error, info},
    sqlx::{Pool, Sqlite},
    std::{thread, time::Duration},
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
            pub pool: Arc<Pool<Sqlite>>,
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
                    provide_context(app_state.pool.clone());
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
            let shared_pool = Arc::new(pool);

            sqlx::query(
                "CREATE TABLE devices (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    ip TEXT NOT NULL UNIQUE,
                    battery_percentage TEXT,
                    power_state TEXT NOT NULL
                )",
            )
            .execute(&*shared_pool.clone())
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
                pool: shared_pool.clone(),
            };

            let iron_nest_router = Router::new()
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

            let shared_pool_clone = shared_pool.clone();
            thread::spawn(move || {
                println!("Running discovery thread for tp-link devices");
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    loop {
                        let tp_link_devices = discover_devices().await;
                        let mut devices: Vec<Device> = Vec::new();

                        for device in tp_link_devices.iter() {
                            for data in device {
                                if let Some(ip) = &data.ip {
                                    devices.push(Device {
                                        id: 0,
                                        name: data.alias.clone(),
                                        ip: ip.clone().to_string(),
                                        state: data.relay_state.to_string(),
                                    });
                                }
                            }
                        }

                        insert_devices_into_db(shared_pool_clone.clone(), &devices).await.unwrap();
                        tokio::time::sleep(Duration::from_secs(300)).await;
                    }
                });
            });

            let shared_pool_clone = shared_pool.clone();
            thread::spawn(move || {
                println!("Running discovery thread for roku devices");
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    loop {
                        let roku_devices = roku_discover().await;
                        let mut devices: Vec<Device> = Vec::new();

                        for device in roku_devices.iter() {
                            devices.push(Device {
                                id: 0,
                                name: "Roku Tv".to_string(),
                                ip: device.location.to_string(),
                                state: 0.to_string(),
                            });
                        }

                        match insert_devices_into_db(shared_pool_clone.clone(), &devices).await {
                            Ok(_) => {},
                            Err(e) => {
                                print!("{e}");
                            }
                        };
                        tokio::time::sleep(Duration::from_secs(300)).await;
                    }
                });
            });

            tokio::select! {
                e = http_server => error!("HTTP server exiting with error {e:?}"),
                e = ring_auth_refresh_job => error!("Ring auth refresh job exiting with error {e:?}"),
            }
        }
    }
}
