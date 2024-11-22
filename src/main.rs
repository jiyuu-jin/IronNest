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
                    insert_initial_devices_into_db, ring_discovery_job, roku_discovery_job,
                    tplink_discovery_job,
                },
                ring::RingRestClient,
            },
        },
        leptos::get_configuration,
        leptos_axum::{generate_route_list, LeptosRoutes},
        log::{error, info, LevelFilter},
        simple_logger::SimpleLogger,
        sqlx::postgres::PgPoolOptions,
        std::{sync::Arc, time::Duration},
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
    let routes = generate_route_list(App);

    let ring_rest_client = Arc::new(RingRestClient::new(shared_pool.clone()).await);

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

    let auth_ring_rest_client = ring_rest_client.clone();
    let _ring_auth_refresh_job = tokio::task::spawn(async move {
        let six_hours = chrono::Duration::hours(6).to_std().unwrap();
        let mut interval = tokio::time::interval(six_hours);
        loop {
            interval.tick().await;

            info!("Refreshing Ring auth token");
            auth_ring_rest_client.refresh_auth_token().await;
        }
    });
    // insert initial devices
    insert_initial_devices_into_db(shared_pool.clone())
        .await
        .unwrap();

    ring_discovery_job(shared_pool.clone(), ring_rest_client.clone());
    tplink_discovery_job(shared_pool.clone());
    roku_discovery_job(shared_pool.clone());

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
