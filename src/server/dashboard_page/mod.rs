use {crate::integrations::iron_nest::types::Device, leptos::prelude::*};

#[server(GetDevices)]
pub async fn get_devices() -> Result<Vec<Device>, ServerFnError> {
    use {
        crate::integrations::iron_nest::types::Device,
        sqlx::{PgPool, Postgres},
    };

    let pool = use_context::<PgPool>().unwrap();

    let query = "
        SELECT id, name, device_type, ip, power_state, battery_percentage, last_seen, mac_address, child_id 
        FROM device
        ORDER BY name
    ";
    sqlx::query_as::<Postgres, Device>(query)
        .fetch_all(&pool)
        .await
        .map_err(Into::into)
}

#[server(RefreshDevices)]
pub async fn refresh_devices() -> Result<(), ServerFnError> {
    use {crate::integrations::iron_nest::refresh_tplink_devices, sqlx::PgPool};

    let pool = use_context::<PgPool>().unwrap();
    refresh_tplink_devices(pool).await;
    Ok(())
}
