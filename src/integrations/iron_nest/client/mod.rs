use {
    super::types::Device,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
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
