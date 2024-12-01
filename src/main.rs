#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use {
        axum::{routing::get, Router},
        dotenv::dotenv,
        iron_nest::{
            components::layout::App,
            fileserv::file_and_error_handler,
            handlers::roku_keypress_handler,
            integrations::{
                iron_nest::{
                    client::{leptos_routes_handler, server_fn_handler, AppState},
                    cron::CronClient,
                    run_devices_tasks,
                },
                ring::RingRestClient,
            },
        },
        leptos::get_configuration,
        leptos_axum::{generate_route_list, LeptosRoutes},
        log::{error, LevelFilter},
        simple_logger::SimpleLogger,
        sqlx::postgres::PgPoolOptions,
        std::{collections::HashMap, sync::Arc},
        tokio::sync::RwLock,
    };

    dotenv().ok();

    let postgres_uri = std::env::var("POSTGRES_URI")
        .unwrap_or("postgres://postgres:password@127.0.0.1:5433/postgres".to_string());
    println!("postgres_uri: {postgres_uri}");
    let shared_pool = PgPoolOptions::new().connect(&postgres_uri).await.unwrap();
    sqlx::migrate!("./migrations")
        .run(&shared_pool)
        .await
        .unwrap();

    SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .with_module_level("iron_nest", LevelFilter::Debug)
        .init()
        .expect("couldn't initialize logging");

    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let ring_rest_client = Arc::new(RingRestClient::new(shared_pool.clone()).await);
    let control_senders = Arc::new(RwLock::new(HashMap::new()));
    let app_state = AppState {
        leptos_options,
        ring_rest_client: ring_rest_client.clone(),
        pool: shared_pool.clone(),
        cron_client: CronClient::new().await,
        control_senders: control_senders.clone(),
    };

    app_state
        .cron_client
        .schedule_tasks(&shared_pool)
        .await
        .unwrap();

    let iron_nest_router = Router::new()
        .route("/roku/:device_id/keypress/:key", get(roku_keypress_handler))
        .with_state(ring_rest_client.clone());

    let routes = generate_route_list(App);
    let app = Router::new()
        .route(
            "/api/*fn_name",
            get(server_fn_handler).post(server_fn_handler),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .nest("/api", iron_nest_router)
        .fallback(file_and_error_handler)
        .with_state(app_state);

    run_devices_tasks(ring_rest_client, shared_pool, control_senders)
        .await
        .unwrap();

    let http_server = {
        log::info!("listening on http://{}", &addr);
        axum::Server::bind(&addr).serve(app.into_make_service())
    };

    tokio::select! {
        e = http_server => error!("HTTP server exiting with error {e:?}")
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    unimplemented!("This function is to make clippy happy")
}
