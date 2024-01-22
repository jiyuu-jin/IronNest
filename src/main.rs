use {
    iron_nest::{
        components::layout::App,
        handlers::roku_keypress_handler,
        integrations::{
            efuy::{eufy_login, get_devices},
            iron_nest::{
                client::insert_devices_into_db,
                create_db_tables, extract_ip, insert_cameras_into_db,
                types::{Device, DeviceType},
            },
            ring::{get_ring_camera, types::DevicesRes, RingRestClient},
            roku::{roku_discover, roku_get_device_info},
            tplink::{discover_devices, types::DeviceData},
        },
    },
    log::{error, info},
    sqlx::{Pool, Sqlite},
    std::time::Duration,
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
                    provide_context(app_state.pool.clone());
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

            create_db_tables(shared_pool.clone()).await;
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

            let auth_ring_rest_client = ring_rest_client.clone();
            let ring_auth_refresh_job = tokio::task::spawn(async move {
                let six_hours = chrono::Duration::hours(6).to_std().unwrap();
                let mut interval = tokio::time::interval(six_hours);
                loop {
                    interval.tick().await;

                    info!("Refreshing Ring auth token");
                    auth_ring_rest_client.refresh_auth_token().await;
                }
            });

            let shared_pool_clone1 = shared_pool.clone();
            let discovery_ring_client = ring_rest_client.clone();
            let ring_device_discovery_job = tokio::task::spawn(async move {
                let five_minutes = chrono::Duration::minutes(5).to_std().unwrap();
                let mut interval = tokio::time::interval(five_minutes);

                loop {
                    interval.tick().await;

                    info!("Refreshing Ring Device Data");
                    let ring_devices = match discovery_ring_client.get_devices().await {
                        Ok(data) => data,
                        Err(_) => DevicesRes{
                            doorbots: Vec::new(),
                            authorized_doorbots: Vec::new(),
                        },
                    };

                    let doorbots = ring_devices
                    .doorbots
                    .into_iter()
                    .chain(ring_devices.authorized_doorbots.into_iter())
                    .collect::<Vec<_>>();

                    let mut cameras = Vec::with_capacity(20);
                    for doorbot in doorbots.iter() {
                        cameras.push(get_ring_camera(&ring_rest_client, doorbot).await)
                    }

                    let mut devices = Vec::with_capacity(20);
                    for camera in cameras.iter() {
                        devices.push(Device{
                            id: 0,
                            name: camera.description.to_string(),
                            ip: camera.id.to_string(),
                            device_type: DeviceType::RingDoorbell,
                            power_state: 1,
                            battery_percentage: camera.health,
                        });
                    }
                    match insert_cameras_into_db(shared_pool_clone1.clone(), &cameras).await {
                        Ok(_) => print!("success"),
                        Err(err) => error!("{err}"),
                    }
                    match insert_devices_into_db(shared_pool_clone1.clone(), &devices).await {
                        Ok(_) => print!("success"),
                        Err(err) => error!("{err}"),
                    }
                }
            });

            let http_server = {
                log::info!("listening on http://{}", &addr);
                axum::Server::bind(&addr).serve(app.into_make_service())
            };

            tokio::task::spawn({
                let shared_pool = shared_pool.clone();
                async move {
                    loop {
                        match discover_devices().await {
                            Ok(tp_link_devices) => {
                                let mut devices: Vec<Device> = Vec::new();

                                for device_data in tp_link_devices {
                                    match device_data {
                                        DeviceData::SmartPlug(data) => {
                                            if let Some(ip) = data.ip {
                                                devices.push(Device {
                                                    id: 0,
                                                    name: data.alias,
                                                    device_type: DeviceType::SmartPlug,
                                                    ip: ip.to_string(),
                                                    power_state: data.relay_state,
                                                    battery_percentage: 0,
                                                });
                                            }
                                        }
                                        DeviceData::SmartLight(data) => {
                                            if let Some(ip) = data.ip {
                                                devices.push(Device {
                                                    id: 0,
                                                    name: data.alias,
                                                    device_type: DeviceType::SmartLight,
                                                    ip: ip.to_string(),
                                                    power_state: data.light_state.on_off,
                                                    battery_percentage: 0,
                                                });
                                            }
                                        }
                                    }
                                }
                                insert_devices_into_db(shared_pool.clone(), &devices).await.unwrap();
                            },
                            Err(e) => {
                                eprintln!("Error discovering devices: {}", e);
                            }
                        }
                        tokio::time::sleep(Duration::from_secs(20)).await;
                    }
                }
            });

            tokio::task::spawn(async move {
                println!("Running discovery thread for roku devices");
                loop {
                    let roku_devices = roku_discover().await;
                    let mut devices: Vec<Device> = Vec::new();

                    for device in roku_devices.iter() {
                        let ip = extract_ip(&device.location).unwrap();
                        let device_info = roku_get_device_info(&ip).await;
                        let power_state = if device_info.power_mode == "PowerOn" { 1 } else {0};
                        devices.push(Device {
                            id: 0,
                            name: device_info.user_device_name,
                            device_type: DeviceType::RokuTv,
                            ip,
                            power_state,
                            battery_percentage: 0,
                        });
                    }

                    match insert_devices_into_db(shared_pool.clone(), &devices).await {
                        Ok(_) => {},
                        Err(e) => {
                            print!("{e}");
                        }
                    };
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            });

            // tokio::task::spawn(async move {
            //     println!("Running discovery thread for eufy devices");
            //     let auth_token = eufy_login().await;
            //     get_devices(auth_token).await;
            //     tokio::time::sleep(Duration::from_secs(300)).await;
            // });

            tokio::select! {
                e = http_server => error!("HTTP server exiting with error {e:?}"),
                e = ring_auth_refresh_job => error!("Ring auth refresh job exiting with error {e:?}"),
                e = ring_device_discovery_job => error!("Ring device discovery job exiting with error {e:?}")
            }
        }
    }
}
