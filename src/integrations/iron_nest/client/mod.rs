use {
    super::types::Device,
    crate::integrations::{
        roku::{roku_launch_app, roku_search, roku_send_keypress},
        tplink::{
            tplink_set_light_brightness, tplink_turn_light_on_off, tplink_turn_plug_off,
            tplink_turn_plug_on,
        },
    },
    serde_json::{json, Value},
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio::task,
    tokio_cron_scheduler::{Job, JobScheduler},
    url::Url,
};

pub async fn insert_devices_into_db(
    pool: Arc<Pool<Sqlite>>,
    devices: &Vec<Device>,
) -> Result<(), sqlx::Error> {
    for device in devices {
        sqlx::query(
            "INSERT OR REPLACE INTO devices (name, device_type, battery_percentage, ip, power_state) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&device.name)
        .bind(&device.device_type)
        .bind(&device.battery_percentage.to_string())
        .bind(&device.ip)
        .bind(&device.state.to_string())
        .execute(&*pool)
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
    function_name: String,
    function_args: serde_json::Value,
    schedule: &str,
) {
    let sched = JobScheduler::new().await.unwrap();

    sched
        .add(
            Job::new(schedule, move |_uuid, _l| {
                let function_name = function_name.clone();
                let function_args = function_args.clone();

                task::spawn(async move {
                    println!("Calling {}", function_name);
                    execute_function(function_name, function_args).await;
                });
            })
            .unwrap(),
        )
        .await
        .unwrap();
}

pub async fn execute_function(function_name: String, function_args: serde_json::Value) -> Value {
    match function_name.as_str() {
        "roku_send_keypress" => {
            let key = function_args["key"]
                .to_string()
                .trim_matches('"')
                .to_string();
            let ip = function_args["ip"]
                .to_string()
                .trim_matches('"')
                .to_string();
            roku_send_keypress(&ip, &key).await
        }
        "tplink_turn_plug_on" => {
            let ip = function_args["ip"].to_string();
            tplink_turn_plug_on(&ip).await;
            json!({
                "message": "success"
            })
        }
        "tplink_turn_plug_off" => {
            let ip = function_args["ip"].to_string();
            tplink_turn_plug_off(&ip).await;
            json!({
                "message": "success"
            })
        }
        "tplink_turn_light_on_off" => {
            let ip = function_args["ip"].to_string();
            let state: u8 = function_args["state"].to_string().parse().unwrap();
            tplink_turn_light_on_off(&ip, state).await;
            json!({
                "message": "success"
            })
        }
        "tplink_set_light_brightness" => {
            let ip = function_args["ip"].to_string();
            let brightness: u8 = function_args["brightness"].to_string().parse().unwrap();
            tplink_set_light_brightness(&ip, brightness).await;
            json!({
                "message": "success"
            })
        }
        "roku_search" => {
            let query = function_args["query"].to_string();
            let ip = function_args["ip"].to_string();
            roku_search(&ip, &query).await
        }
        "roku_launch_app" => {
            let app_id = function_args["app_id"].to_string();
            let ip = function_args["ip"].to_string();
            roku_launch_app(&ip, &app_id).await
        }
        &_ => todo!(),
    }
}
