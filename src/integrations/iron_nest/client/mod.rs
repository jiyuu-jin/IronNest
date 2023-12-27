use {
    super::types::Device,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
};

pub async fn insert_devices_into_db(
    pool: Arc<Pool<Sqlite>>,
    devices: &Vec<Device>,
) -> Result<(), sqlx::Error> {
    for device in devices {
        sqlx::query(
            "INSERT OR REPLACE INTO devices (name, device_type, ip, power_state) VALUES (?, ?, ?, ?)",
        )
        .bind(&device.name)
        .bind(&device.device_type)
        .bind(&device.ip)
        .bind(&device.state.to_string())
        .execute(&*pool)
        .await?;
    }

    Ok(())
}
