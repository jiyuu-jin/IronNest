use leptos::*;

#[server(HandleSmartPlugToggle)]
pub async fn handle_smart_plug_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on},
        sqlx::PgPool,
    };

    let pool = use_context::<PgPool>().unwrap();
    let query = "
        UPDATE devices
        SET power_state = $1
        WHERE ip = $2
    ";
    sqlx::query(query)
        .bind(if state { 1 } else { 0 })
        .bind(&ip)
        .execute(&pool)
        .await?;
    if state {
        tplink_turn_plug_on(&ip).await;
    } else {
        tplink_turn_plug_off(&ip).await;
    }
    Ok(())
}

#[server(HandleSmartLightToggle)]
pub async fn handle_smart_light_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use {crate::integrations::tplink::tplink_turn_light_on_off, sqlx::PgPool};

    let state = if state { 1 } else { 0 };

    let pool = use_context::<PgPool>().unwrap();
    let query = "
        UPDATE devices
        SET power_state = $1
        WHERE ip = $2
    ";
    println!("state: {state}");
    println!("ip: {ip}");
    sqlx::query(query)
        .bind(state as i32)
        .bind(&ip)
        .execute(&pool)
        .await?;
    tplink_turn_light_on_off(&ip, state).await;
    Ok(())
}

#[server(HandleSmartLightBrightness)]
pub async fn handle_smart_light_brightness(
    brightness: u8,
    ip: String,
) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_set_light_brightness;
    tplink_set_light_brightness(&ip, brightness).await;
    Ok(())
}

#[server(HandleSmartLightSaturation)]
pub async fn handle_smart_light_saturation(
    saturation: u8,
    ip: String,
) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_set_light_saturation;
    tplink_set_light_saturation(&ip, saturation).await;
    Ok(())
}
