use {
    super::types::{AuthState, Device, DeviceType},
    crate::{
        components::layout::App,
        integrations::{
            efuy,
            ring::{
                client::RingRestClient,
                get_ring_camera,
                types::{DevicesRes, RingCamera},
            },
            roku::{
                roku_discover, roku_get_device_info, roku_launch_app, roku_search,
                roku_send_keypress,
            },
            stoplight::toggle_stoplight,
            tplink::{
                discover_devices, tplink_set_light_brightness, tplink_turn_light_on_off,
                tplink_turn_plug_off, tplink_turn_plug_on, types::DeviceData,
            },
            tuya::{get_devices, get_refresh_token},
        },
    },
    axum::{
        body::Body as AxumBody,
        extract::{FromRef, Path, RawQuery, State},
        response::{IntoResponse, Response},
    },
    chrono::Utc,
    http::Request,
    leptos::{logging::log, provide_context, LeptosOptions},
    leptos_axum::handle_server_fns_with_context,
    log::{error, info},
    reqwest::header::HeaderMap,
    serde_json::{json, Value},
    sqlx::{PgPool, Pool, Postgres},
    std::sync::Arc,
    tokio_cron_scheduler::{Job, JobScheduler},
    url::Url,
};

pub async fn insert_devices_into_db(
    pool: PgPool,
    devices: &Vec<Device>,
) -> Result<(), sqlx::Error> {
    for device in devices {
        let query = "
            INSERT INTO devices (
                name,
                device_type,
                battery_percentage,
                ip,
                power_state,
                last_seen
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (ip) DO UPDATE
            SET name=$1,
                device_type=$2,
                battery_percentage=$3,
                ip=$4,
                power_state=$5,
                last_seen=$6
        ";
        sqlx::query(query)
            .bind(&device.name)
            .bind(&device.device_type)
            .bind(device.battery_percentage)
            .bind(&device.ip)
            .bind(device.power_state)
            .bind(device.last_seen)
            .execute(&pool)
            .await?;
    }

    Ok(())
}

pub async fn insert_initial_devices_into_db(pool: PgPool) -> Result<(), sqlx::Error> {
    insert_devices_into_db(
        pool,
        &vec![Device {
            name: "Living Room Stoplight".to_owned(),
            device_type: DeviceType::Stoplight,
            id: 0,
            ip: "0.0.0.1".to_owned(),
            battery_percentage: 0,
            power_state: 0,
            last_seen: Utc::now(),
        }],
    )
    .await
    .unwrap();

    Ok(())
}

pub fn extract_ip(url_str: &str) -> Result<String, url::ParseError> {
    let url = Url::parse(url_str)?;
    match url.host_str() {
        Some(host) => Ok(host.to_string()),
        None => Err(url::ParseError::EmptyHost),
    }
}

pub async fn schedule_task(
    function_name: Arc<String>,
    function_args: Arc<Value>,
    schedule: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let sched = JobScheduler::new().await?;
    sched
        .add(
            Job::new_async(schedule, move |_uuid, mut _l| {
                let function_name_clone = function_name.clone();
                let function_args_clone = function_args.clone();

                Box::pin(async move {
                    let fn_name = (*function_name_clone).clone();
                    let fn_args = (*function_args_clone).clone();

                    println!("Calling {}", fn_name);
                    execute_function(fn_name, fn_args).await;
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();
    sched.start().await?;
    Ok(())
}

pub async fn execute_function(function_name: String, function_args: serde_json::Value) -> Value {
    match function_name.as_str() {
        "roku_send_keypress" => {
            let key = function_args["key"].as_str().unwrap();
            let ip = function_args["ip"].as_str().unwrap();
            roku_send_keypress(ip, key).await
        }
        "tplink_turn_plug_on" => {
            let ip = function_args["ip"].as_str().unwrap();
            tplink_turn_plug_on(ip).await;
            json!({
                "message": "success"
            })
        }
        "tplink_turn_plug_off" => {
            let ip = function_args["ip"].as_str().unwrap();
            tplink_turn_plug_off(ip).await;
            json!({
                "message": "success"
            })
        }
        "tplink_turn_light_on_off" => {
            let ip = function_args["ip"].as_str().unwrap();
            let state: u8 = function_args["state"].as_str().unwrap().parse().unwrap();
            tplink_turn_light_on_off(ip, state).await;
            json!({
                "message": "success"
            })
        }
        "tplink_set_light_brightness" => {
            let ip = function_args["ip"].as_str().unwrap();
            let brightness: u8 = function_args["brightness"]
                .as_u64()
                .unwrap()
                .try_into()
                .unwrap();
            tplink_set_light_brightness(ip, brightness).await;
            json!({
                "message": "success"
            })
        }
        "roku_search" => {
            let query = function_args["query"].as_str().unwrap();
            let ip = function_args["ip"].as_str().unwrap();
            roku_search(ip, query).await
        }
        "roku_launch_app" => {
            let app_id = function_args["app_id"].as_str().unwrap();
            let ip = function_args["ip"].as_str().unwrap();
            roku_launch_app(ip, app_id).await
        }
        "stoplight_toggle" => {
            let color = function_args["color"].as_str().unwrap();
            let result = toggle_stoplight(color).await.is_ok();
            json!({"success": result})
        }
        &_ => todo!(),
    }
}

pub async fn insert_cameras_into_db(
    pool: PgPool,
    cameras: &[RingCamera],
) -> Result<(), sqlx::Error> {
    info!("Inserting cameras into db");
    for camera in cameras.iter() {
        sqlx::query(
            "
            INSERT INTO ring_cameras (id, description, snapshot_image, snapshot_timestamp, health) 
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                description = EXCLUDED.description,
                snapshot_image = EXCLUDED.snapshot_image,
                snapshot_timestamp = EXCLUDED.snapshot_timestamp,
                health = EXCLUDED.health
            ",
        )
        .bind(camera.id)
        .bind(&camera.description)
        .bind(&camera.snapshot.image)
        .bind(camera.snapshot.timestamp)
        .bind(camera.health)
        .execute(&pool)
        .await?;

        for video_item in camera.videos.video_search.iter() {
            sqlx::query(
                "
                INSERT INTO ring_video_item (ding_id, camera_id, created_at, hq_url) 
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (ding_id) DO UPDATE SET
                    camera_id = EXCLUDED.camera_id,
                    created_at = EXCLUDED.created_at,
                    hq_url = EXCLUDED.hq_url
                ",
            )
            .bind(&video_item.ding_id)
            .bind(camera.id)
            .bind(video_item.created_at.to_string())
            .bind(&video_item.hq_url)
            .execute(&pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn insert_auth(pool: PgPool, name: &str, state: AuthState) {
    let dt = Utc::now();
    let query = "
        INSERT INTO auth (name, auth_token, refresh_token, hardware_id, last_login) 
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT(name) DO UPDATE SET
            auth_token = EXCLUDED.auth_token,
            refresh_token = EXCLUDED.refresh_token,
            hardware_id = EXCLUDED.hardware_id,
            last_login = EXCLUDED.last_login;
    ";

    sqlx::query(query)
        .bind(name)
        .bind(&state.auth_token)
        .bind(&state.refresh_token)
        .bind(&state.hardware_id)
        .bind(dt)
        .execute(&pool)
        .await
        .unwrap();
}

pub async fn get_auth_from_db(pool: PgPool, name: &str) -> AuthState {
    let query = "
        SELECT hardware_id, auth_token, refresh_token 
        FROM auth
        WHERE name=$1
    ";

    let auth_query = sqlx::query_as::<_, AuthState>(query)
        .bind(name)
        .fetch_one(&pool)
        .await;

    match auth_query {
        Ok(state) => state,
        Err(err) => {
            error!("{err}");
            AuthState {
                hardware_id: "".to_string(),
                auth_token: "".to_string(),
                refresh_token: "".to_string(),
            }
        }
    }
}

pub async fn leptos_routes_handler(
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

pub async fn server_fn_handler(
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

#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub ring_rest_client: Arc<RingRestClient>,
    pub pool: PgPool,
}

pub fn tuya_auth_job(shared_pool: Pool<Postgres>) {
    let shared_pool_1 = shared_pool.clone();
    tokio::task::spawn(async move {
        println!("Running thread for tuya auth");
        let tuya_auth = get_auth_from_db(shared_pool_1.clone(), "tuya").await;

        if !tuya_auth.refresh_token.is_empty() {
            println!("Found a refresh_token, refreshing auth_token");
            let res = get_refresh_token().await.unwrap();
            insert_auth(
                shared_pool_1,
                "tuya",
                AuthState {
                    refresh_token: res.result.refresh_token,
                    hardware_id: res.result.uid,
                    auth_token: res.result.access_token,
                },
            )
            .await;
        } else {
            println!("No refresh_token found, getting a new one");
            let res = get_refresh_token().await.unwrap();
            insert_auth(
                shared_pool_1,
                "tuya",
                AuthState {
                    refresh_token: res.result.refresh_token,
                    hardware_id: res.result.uid,
                    auth_token: res.result.access_token,
                },
            )
            .await;
        }
        tokio::time::sleep(chrono::Duration::hours(1).to_std().unwrap()).await;
    });
}

pub fn tuya_discovery_job(shared_pool: Pool<Postgres>) {
    let shared_pool_1 = shared_pool.clone();
    tokio::task::spawn(async move {
        println!("Running thread for tuya discovery");
        let tuya_auth = get_auth_from_db(shared_pool_1.clone(), "tuya").await;
        if !tuya_auth.auth_token.is_empty() {
            // let res = get_user_id("ebbd589a10538c471dbeaf", &tuya_auth.auth_token).await;
            let res = get_devices("az17063780590351Cr1b", &tuya_auth.auth_token)
                .await
                .unwrap();
            println!("{:?}", res);
            let devices = res
                .result
                .iter()
                .map(|device| Device {
                    id: 0,
                    name: device.name.clone(),
                    device_type: DeviceType::SmartLight,
                    ip: device.ip.clone(),
                    power_state: 0,
                    battery_percentage: 0,
                    last_seen: Utc::now(),
                })
                .collect();

            insert_devices_into_db(shared_pool_1.clone(), &devices)
                .await
                .unwrap();
        }
        tokio::time::sleep(chrono::Duration::hours(1).to_std().unwrap()).await;
    });
}

pub fn eufy_auth_job(shared_pool: Pool<Postgres>) {
    let shared_pool_1 = shared_pool.clone();
    tokio::task::spawn(async move {
        println!("Running thread for eufy auth");
        let eufy_auth = get_auth_from_db(shared_pool_1.clone(), "eufy").await;
        if !eufy_auth.refresh_token.is_empty() {
            println!("Found a refresh_token, refreshing auth_token");
        } else {
            println!("No refresh_token found, getting a new one");
            let res = efuy::eufy_login().await;
            insert_auth(
                shared_pool_1,
                "eufy",
                AuthState {
                    refresh_token: res.data.auth_token.to_owned(),
                    hardware_id: res.data.user_id,
                    auth_token: res.data.auth_token,
                },
            )
            .await;
        }

        tokio::time::sleep(chrono::Duration::hours(1).to_std().unwrap()).await;
    });
}

pub fn eufy_discovery_job(shared_pool: Pool<Postgres>) {
    let shared_pool_1 = shared_pool.clone();
    tokio::task::spawn(async move {
        println!("Running thread for eufy discovery");
        let eufy_auth = get_auth_from_db(shared_pool_1.clone(), "eufy").await;
        if !eufy_auth.auth_token.is_empty() {
            efuy::get_devices(eufy_auth.auth_token).await;
        }
        tokio::time::sleep(chrono::Duration::hours(1).to_std().unwrap()).await;
    });
}

pub fn roku_discovery_job(shared_pool: Pool<Postgres>) {
    tokio::task::spawn(async move {
        println!("Running discovery thread for roku devices");
        loop {
            let roku_devices = roku_discover().await;
            let mut devices: Vec<Device> = Vec::new();

            for device in roku_devices.iter() {
                let ip = extract_ip(&device.location).unwrap();
                let device_info = roku_get_device_info(&ip).await;
                let power_state = if device_info.power_mode == "PowerOn" {
                    1
                } else {
                    0
                };
                devices.push(Device {
                    id: 0,
                    name: device_info.user_device_name,
                    device_type: DeviceType::RokuTv,
                    ip,
                    power_state,
                    battery_percentage: 0,
                    last_seen: Utc::now(),
                });
            }

            match insert_devices_into_db(shared_pool.clone(), &devices).await {
                Ok(_) => {}
                Err(e) => {
                    print!("{e}");
                }
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}

pub fn tplink_discovery_job(shared_pool: Pool<Postgres>) {
    tokio::task::spawn({
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
                                            last_seen: Utc::now(),
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
                                            last_seen: Utc::now(),
                                        });
                                    }
                                }
                            }
                        }
                        insert_devices_into_db(shared_pool.clone(), &devices)
                            .await
                            .unwrap();
                    }
                    Err(e) => {
                        eprintln!("Error discovering devices: {}", e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
    });
}

pub fn ring_discovery_job(shared_pool: Pool<Postgres>, ring_rest_client: Arc<RingRestClient>) {
    let _ring_device_discovery_job = tokio::task::spawn(async move {
        let five_minutes = chrono::Duration::minutes(5).to_std().unwrap();
        let mut interval = tokio::time::interval(five_minutes);

        loop {
            interval.tick().await;

            info!("Refreshing Ring Device Data");
            let ring_devices = match ring_rest_client.get_devices().await {
                Ok(data) => data,
                Err(_) => DevicesRes {
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
                devices.push(Device {
                    id: 0,
                    name: camera.description.to_string(),
                    ip: camera.id.to_string(),
                    device_type: DeviceType::RingDoorbell,
                    power_state: 1,
                    battery_percentage: camera.health,
                    last_seen: Utc::now(),
                });
            }
            match insert_cameras_into_db(shared_pool.clone(), &cameras).await {
                Ok(_) => print!("success"),
                Err(err) => error!("{err}"),
            }
            match insert_devices_into_db(shared_pool.clone(), &devices).await {
                Ok(_) => print!("success"),
                Err(err) => error!("{err}"),
            }
        }
    });
}
