use {crate::integrations::iron_nest::types::Device, leptos::*};

#[server(GetDevices)]
pub async fn get_devices() -> Result<Vec<Device>, ServerFnError> {
    use {
        crate::integrations::iron_nest::types::Device,
        sqlx::{PgPool, Postgres},
    };

    let pool = use_context::<PgPool>().unwrap();

    let query = "
        SELECT id, name, device_type, ip, power_state, 0 AS battery_percentage 
        FROM devices
    ";
    sqlx::query_as::<Postgres, Device>(query)
        .fetch_all(&pool)
        .await
        .map_err(Into::into)
}
