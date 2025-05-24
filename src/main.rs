#[cfg(feature = "ssr")]
mod shell;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use {
        axum::{routing::get, Router},
        dotenv::dotenv,
        iron_nest::{
            components::layout::App,
            handlers::roku_keypress_handler,
            integrations::{
                iron_nest::{client::AppState, cron::CronClient, mish::{create_mish_state_modification_bus, register_native_queries}, run_devices_tasks},
                ring::RingRestClient,
                tplink::tplink_kasa_get_energy_usage,
            },
        },
        leptos::prelude::*,
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

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let ring_rest_client = Arc::new(RingRestClient::new(shared_pool.clone()).await);
    let control_senders = Arc::new(RwLock::new(HashMap::new()));
    let (mish_state_modification_bus_sender, mish_state_modification_bus_receiver) =
        create_mish_state_modification_bus();
    let app_state = AppState {
        leptos_options: leptos_options.clone(),
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
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                provide_context(app_state.ring_rest_client.clone());
                provide_context(app_state.pool.clone());
                provide_context(app_state.cron_client.clone());
                provide_context(app_state.control_senders.clone());
                provide_context(mish_state_modification_bus_sender.clone());
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell::shell(leptos_options.clone())
            },
        )
        .nest("/api", iron_nest_router)
        .fallback(leptos_axum::file_and_error_handler(shell::shell))
        .with_state(leptos_options);

    run_devices_tasks(ring_rest_client, &shared_pool, control_senders)
        .await
        .unwrap();

    tokio::spawn(async move {
        register_native_queries(mish_state_modification_bus_receiver).await;
    });

    tplink_kasa_get_energy_usage("10.0.0.223", "1")
        .await
        .unwrap();
    tplink_kasa_get_energy_usage("10.0.0.223", "1")
        .await
        .unwrap();

    let http_server = {
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        log::info!("listening on http://{}", &addr);
        axum::serve(listener, app.into_make_service())
    };

    tokio::select! {
        e = http_server => error!("HTTP server exiting with error {e:?}")
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    unimplemented!("This function is to make clippy happy")
}
