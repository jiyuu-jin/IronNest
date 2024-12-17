use {
    super::{
        cron::CronClient,
        shared::get_default_integrations,
        types::{AuthState, ControlMessage, Device, DeviceType, Integration},
    },
    crate::integrations::{
        efuy,
        ring::{
            client::RingRestClient,
            get_ring_camera,
            types::{DevicesRes, RingCamera},
        },
        roku::{
            roku_discover, roku_get_device_info, roku_launch_app, roku_search, roku_send_keypress,
        },
        stoplight::toggle_stoplight,
        tplink::{
            discover_devices, tplink_set_dimmer_brightness, tplink_set_light_brightness,
            tplink_turn_light_on_off, tplink_turn_plug_off, tplink_turn_plug_on, types::DeviceData,
        },
        tuya::{discover_tuya_devices, get_devices, get_refresh_token, types::TuyaDeviceResResult},
    },
    chrono::Utc,
    leptos::prelude::*,
    log::{error, info},
    serde_json::{json, Value},
    sqlx::PgPool,
    std::{collections::HashMap, net::Ipv4Addr, sync::Arc},
    tokio::sync::{
        mpsc::{self, Receiver, Sender},
        RwLock,
    },
    tokio_cron_scheduler::{Job, JobScheduler},
    url::Url,
};

pub async fn insert_devices_into_db(
    pool: &PgPool,
    devices: &Vec<Device>,
) -> Result<(), sqlx::Error> {
    for device in devices {
        println!("insert_devices_into_db device {:?}", device);
        let query = "
            INSERT INTO device (
                name,
                device_type,
                battery_percentage,
                ip,
                power_state,
                last_seen,
                child_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT ON CONSTRAINT unique_ip_child_id DO UPDATE
            SET name=$1,
                device_type=$2,
                battery_percentage=$3,
                ip=$4,
                power_state=$5,
                last_seen=$6,
                child_id=$7
        ";
        sqlx::query(query)
            .bind(&device.name)
            .bind(&device.device_type)
            .bind(device.battery_percentage)
            .bind(&device.ip)
            .bind(device.power_state)
            .bind(device.last_seen)
            .bind(&device.child_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

pub async fn insert_initial_devices_into_db(pool: &PgPool) -> Result<(), sqlx::Error> {
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
            mac_address: None,
            child_id: None,
        }],
    )
    .await
    .unwrap();

    Ok(())
}

pub async fn insert_integrations_into_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    for integration in get_default_integrations() {
        let query = "
            INSERT INTO integration (
                name,
                enabled,
                image
            ) VALUES ($1, $2, $3)
            ON CONFLICT (name) DO NOTHING;
        ";
        sqlx::query(query)
            .bind(&integration.name)
            .bind(integration.enabled)
            .bind(integration.image)
            .execute(pool)
            .await?;
    }

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
        "tplink_set_dimmer_brightness" => {
            let ip = function_args["ip"].as_str().unwrap();
            let brightness: u8 = function_args["brightness"]
                .as_u64()
                .unwrap()
                .try_into()
                .unwrap();
            tplink_set_dimmer_brightness(ip, &brightness).await;
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

pub async fn insert_tuya_device_keys(
    pool: PgPool,
    device: TuyaDeviceResResult,
) -> Result<(), sqlx::Error> {
    let query = "
        INSERT INTO tuya_device_data (id, local_key) 
        VALUES ($1, $2)
        ON CONFLICT(id) DO UPDATE SET
            local_key = EXCLUDED.local_key,
    ";

    sqlx::query(query)
        .bind(device.uid)
        .bind(device.local_key)
        .execute(&pool)
        .await?;

    Ok(())
}

pub async fn insert_cameras_into_db(
    pool: &PgPool,
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
        .execute(pool)
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
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn insert_auth(pool: &PgPool, name: &str, state: AuthState) {
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
        .execute(pool)
        .await
        .unwrap();
}

pub async fn get_auth_from_db(pool: &PgPool, name: &str) -> AuthState {
    let query = "
        SELECT hardware_id, auth_token, refresh_token 
        FROM auth
        WHERE name=$1
    ";

    let auth_query = sqlx::query_as::<_, AuthState>(query)
        .bind(name)
        .fetch_one(pool)
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

#[derive(Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub ring_rest_client: Arc<RingRestClient>,
    pub pool: PgPool,
    pub cron_client: CronClient,
    pub control_senders: Arc<RwLock<HashMap<String, Sender<ControlMessage>>>>,
}

pub fn match_control_message(msg: ControlMessage, running: &mut bool) -> bool {
    match msg {
        ControlMessage::Start => {
            println!("Starting discovery job");
            *running = true;
            true
        }
        ControlMessage::Stop => {
            println!("Stopping discovery job");
            *running = false;
            true
        }
        ControlMessage::Shutdown => {
            println!("Shutting down discovery job");
            false
        }
    }
}

pub fn tuya_job(
    shared_pool: PgPool,
    mut control_rx: Receiver<ControlMessage>,
    initial_enabled: bool,
) {
    tokio::task::spawn(async move {
        println!("Running Tuya discovery job");
        let mut auth_interval = tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
        let mut discovery_interval =
            tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
        let mut running = initial_enabled;

        loop {
            tokio::select! {
                _ = auth_interval.tick(), if running => {
                    let tuya_auth = get_auth_from_db(&shared_pool, "tuya").await;
                    if !tuya_auth.refresh_token.is_empty() {
                        println!("Found a refresh_token, refreshing auth_token");
                        let res = get_refresh_token().await.unwrap();
                        insert_auth(
                            &shared_pool,
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
                            &shared_pool,
                            "tuya",
                            AuthState {
                                refresh_token: res.result.refresh_token,
                                hardware_id: res.result.uid,
                                auth_token: res.result.access_token,
                            },
                        )
                        .await;
                    }
                },
                _ = discovery_interval.tick(), if running => {
                    let tuya_auth = get_auth_from_db(&shared_pool, "tuya").await;
                    if !tuya_auth.auth_token.is_empty() {
                        // @TODO refactor user_id to come from db
                        let res = get_devices("az17063780590351Cr1b", &tuya_auth.auth_token)
                            .await
                            .unwrap();
                        println!("{:?}", res);
                        let devices: Vec<Device> = res
                            .result
                            .iter().enumerate()
                            .map(|(index, device)| {
                                let ip = Ipv4Addr::new(0, 0, 0, 0).to_string();
                                Device {
                                    id: 0,
                                    name: device.name.clone(),
                                    device_type: DeviceType::TuyaLight,
                                    ip,
                                    power_state: 0,
                                    battery_percentage: 0,
                                    last_seen: Utc::now(),
                                    mac_address: None,
                                    child_id: Some(index.to_string()),
                                }
                            })
                            .collect();

                        println!("bhap: {:?}", devices);
                        insert_devices_into_db(&shared_pool, &devices)
                            .await
                            .unwrap();

                        // task for local network discovery
                        tokio::task::spawn(async {
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(10),
                                discover_tuya_devices(),
                            ).await {
                                Ok(Ok(_)) => println!("Local Tuya discovery completed."),
                                Ok(Err(e)) => println!("Error during local Tuya discovery: {}", e),
                                Err(_) => println!("Local Tuya discovery timed out."),
                            }
                        });
                    }
                },
                Some(msg) = control_rx.recv() => {
                    println!("Received control message: {:?}", msg);
                    if !match_control_message(msg, &mut running) {
                        break;
                    }
                },
                else => {
                    println!("Control channel closed");
                    break;
                }
            }
        }
    });
}

pub fn eufy_job(
    shared_pool: PgPool,
    mut control_rx: Receiver<ControlMessage>,
    initial_enabled: bool,
) {
    tokio::task::spawn(async move {
        println!("Running Eufy discovery job");
        let mut auth_interval = tokio::time::interval(chrono::Duration::hours(5).to_std().unwrap());
        let mut discovery_interval =
            tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
        let mut running = initial_enabled;

        loop {
            tokio::select! {
                _ = auth_interval.tick(), if running => {
                    let eufy_auth = get_auth_from_db(&shared_pool, "eufy").await;
                    if !eufy_auth.refresh_token.is_empty() {
                        println!("Found a refresh_token, refreshing auth_token");
                        // Add your refresh logic here
                    } else {
                        println!("No refresh_token found, getting a new one");
                        let res = efuy::eufy_login().await;
                        insert_auth(
                            &shared_pool,
                            "eufy",
                            AuthState {
                                refresh_token: res.data.auth_token.to_owned(),
                                hardware_id: res.data.user_id,
                                auth_token: res.data.auth_token,
                            },
                        )
                        .await;
                    }
                },
                _ = discovery_interval.tick(), if running => {
                    let eufy_auth = get_auth_from_db(&shared_pool, "eufy").await;
                    if !eufy_auth.auth_token.is_empty() {
                        efuy::get_devices(eufy_auth.auth_token).await;
                    }
                },
                Some(msg) = control_rx.recv() => {
                    println!("Received control message: {:?}", msg);
                    if !match_control_message(msg, &mut running) {
                        break;
                    }
                },
                else => {
                    println!("Control channel closed");
                    break;
                }
            }
        }
    });
}

pub fn ring_job(
    shared_pool: PgPool,
    ring_rest_client: Arc<RingRestClient>,
    mut control_rx: Receiver<ControlMessage>,
    initial_enabled: bool,
) {
    tokio::task::spawn(async move {
        println!("Running Ring discovery job");
        let mut auth_interval = tokio::time::interval(chrono::Duration::hours(5).to_std().unwrap());
        let mut discovery_interval =
            tokio::time::interval(chrono::Duration::minutes(5).to_std().unwrap());
        let mut running = initial_enabled;

        loop {
            tokio::select! {
                _ = auth_interval.tick(), if running => {
                    info!("Refreshing Ring auth token");
                    ring_rest_client.refresh_auth_token().await;
                },
                _ = discovery_interval.tick(), if running => {
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
                        .chain(ring_devices.authorized_doorbots)
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
                            mac_address: None,
                            child_id: None,
                        });
                    }
                    match insert_cameras_into_db(&shared_pool, &cameras).await {
                        Ok(_) => print!("success"),
                        Err(err) => error!("{err}"),
                    }
                    match insert_devices_into_db(&shared_pool, &devices).await {
                        Ok(_) => print!("success"),
                        Err(err) => error!("{err}"),
                    }
                },
                Some(msg) = control_rx.recv() => {
                    println!("Received control message: {:?}", msg);
                    if !match_control_message(msg, &mut running) {
                        break;
                    }
                },
                else => {
                    println!("Control channel closed");
                    break;
                }
            }
        }
    });
}

pub fn roku_discovery_job(
    shared_pool: PgPool,
    mut control_rx: Receiver<ControlMessage>,
    initial_enabled: bool,
) {
    tokio::task::spawn(async move {
        println!("Running Roku discovery job");
        let mut interval = tokio::time::interval(chrono::Duration::hours(1).to_std().unwrap());
        let mut running = initial_enabled;

        loop {
            tokio::select! {
                _ = interval.tick(), if running => {
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
                            mac_address: None,
                            child_id: None,
                        });
                    }

                    match insert_devices_into_db(&shared_pool, &devices).await {
                        Ok(_) => {}
                        Err(e) => {
                            print!("{e}");
                        }
                    };
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                },
                Some(msg) = control_rx.recv() => {
                    println!("Received control message: {:?}", msg);
                    if !match_control_message(msg, &mut running) {
                        break;
                    }
                }
            }
        }
    });
}

pub async fn refresh_tplink_devices(shared_pool: PgPool) {
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
                                device_type: DeviceType::KasaPlug,
                                ip: ip.to_string(),
                                power_state: data.relay_state,
                                battery_percentage: 0,
                                last_seen: Utc::now(),
                                mac_address: None,
                                child_id: None,
                            });
                        }
                    }
                    DeviceData::SmartLight(data) => {
                        if let Some(ip) = data.ip {
                            devices.push(Device {
                                id: 0,
                                name: data.alias,
                                device_type: DeviceType::KasaLight,
                                ip: ip.to_string(),
                                power_state: data.light_state.on_off,
                                battery_percentage: 0,
                                last_seen: Utc::now(),
                                mac_address: None,
                                child_id: None,
                            });
                        }
                    }
                    DeviceData::SmartDimmer(data) => {
                        if let Some(ip) = data.ip {
                            devices.push(Device {
                                id: 0,
                                name: data.alias,
                                device_type: DeviceType::KasaDimmer,
                                ip: ip.to_string(),
                                power_state: data.relay_state,
                                battery_percentage: 0,
                                last_seen: Utc::now(),
                                mac_address: None,
                                child_id: None,
                            });
                        }
                    }
                    DeviceData::SmartPowerStrip(data) => {
                        if let Some(ip) = data.ip {
                            for outlet in data.children {
                                devices.push(Device {
                                    id: 0,
                                    name: outlet.alias,
                                    device_type: DeviceType::KasaPowerStrip,
                                    ip: ip.to_string(),
                                    power_state: outlet.state,
                                    battery_percentage: 0,
                                    last_seen: Utc::now(),
                                    mac_address: None,
                                    child_id: Some(format!("{}{}", data.device_id, outlet.id)),
                                });
                            }
                        }
                    }
                }
            }
            insert_devices_into_db(&shared_pool, &devices)
                .await
                .unwrap();
        }
        Err(e) => {
            eprintln!("Error discovering devices: {}", e);
        }
    }
}

pub fn tplink_discovery_job(
    shared_pool: PgPool,
    mut control_rx: Receiver<ControlMessage>,
    initial_enabled: bool,
) {
    let shared_pool = shared_pool.clone();
    tokio::task::spawn(async move {
        println!("Running TPlink discovery job");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        let mut running = initial_enabled;

        loop {
            tokio::select! {
                _ = interval.tick(), if running => {
                    refresh_tplink_devices(shared_pool.clone()).await;
                },
                Some(msg) = control_rx.recv() => {
                    println!("Received control message: {:?}", msg);
                    match msg {
                        ControlMessage::Start => {
                            println!("Starting discovery job");
                            running = true;
                        }
                        ControlMessage::Stop => {
                            println!("Stopping discovery job");
                            running = false;
                        }
                        ControlMessage::Shutdown => {
                            println!("Shutting down discovery job");
                            break;
                        }
                    }
                }
            }
        }
    });
}

pub async fn get_integrations(shared_pool: &PgPool) -> Result<Vec<Integration>, sqlx::Error> {
    let query = "
        SELECT id, name, enabled, image 
        FROM integration
    ";
    sqlx::query_as(query).fetch_all(shared_pool).await
}

pub async fn run_devices_tasks(
    ring_rest_client: Arc<RingRestClient>,
    shared_pool: &PgPool,
    control_senders: Arc<RwLock<HashMap<String, Sender<ControlMessage>>>>,
) -> Result<(), sqlx::Error> {
    insert_integrations_into_db(shared_pool).await?;
    insert_initial_devices_into_db(shared_pool).await?;
    let integrations = get_integrations(shared_pool).await.unwrap();

    for integration in integrations {
        match integration.name.as_str() {
            "tplink" => {
                let (tx, rx) = mpsc::channel(10);
                tplink_discovery_job(shared_pool.clone(), rx, integration.enabled);
                let mut senders = control_senders.write().await;
                senders.insert("tplink".to_string(), tx);
            }
            "roku" => {
                let (tx, rx) = mpsc::channel(10);
                roku_discovery_job(shared_pool.clone(), rx, integration.enabled);
                let mut senders = control_senders.write().await;
                senders.insert("roku".to_string(), tx);
            }
            "ring" => {
                let (tx, rx) = mpsc::channel(10);
                ring_job(
                    shared_pool.clone(),
                    ring_rest_client.clone(),
                    rx,
                    integration.enabled,
                );
                let mut senders = control_senders.write().await;
                senders.insert("ring".to_string(), tx);
            }
            "tuya" => {
                let (tx, rx) = mpsc::channel(10);
                tuya_job(shared_pool.clone(), rx, integration.enabled);
                let mut senders = control_senders.write().await;
                senders.insert("tuya".to_string(), tx);
            }
            "eufy" => {
                let (tx, rx) = mpsc::channel(10);
                eufy_job(shared_pool.clone(), rx, integration.enabled);
                let mut senders = control_senders.write().await;
                senders.insert("eufy".to_string(), tx);
            }
            _ => {}
        }
    }
    Ok(())
}

// integrations register their own supported common capabilities
// 'color', 'brightness', 'on_off'
