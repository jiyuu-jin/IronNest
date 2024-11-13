use {
    super::types::{AuthState, Device, DeviceType},
    crate::{
        components::layout::App,
        integrations::{
            ring::{client::RingRestClient, types::RingCamera},
            roku::{roku_launch_app, roku_search, roku_send_keypress},
            stoplight::toggle_stoplight,
            tplink::{
                tplink_set_light_brightness, tplink_turn_light_on_off, tplink_turn_plug_off,
                tplink_turn_plug_on,
            },
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
    sqlx::PgPool,
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
                name, device_type, battery_percentage, ip, power_state 
           ) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (ip) DO UPDATE SET name=$1, device_type=$2, battery_percentage=$3, ip=$4, power_state=$5";
        sqlx::query(query)
            .bind(&device.name)
            .bind(&device.device_type)
            .bind(device.battery_percentage)
            .bind(&device.ip)
            .bind(device.power_state)
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
    info!("Inserting camera into db");
    for camera in cameras.iter() {
        sqlx::query(
            "INSERT OR REPLACE INTO ring_cameras (id, description, snapshot_image, snapshot_timestamp, health) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(camera.id)
        .bind(&camera.description)
        .bind(&camera.snapshot.image)
        .bind(&camera.snapshot.timestamp)
        .bind(camera.health)
        .execute(&pool)
        .await?;

        for video_item in camera.videos.video_search.iter() {
            sqlx::query(
                "INSERT OR REPLACE INTO ring_video_item (ding_id, camera_id, created_at, hq_url) VALUES (?, ?, ?, ?)",
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
