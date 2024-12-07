use leptos::prelude::*;

#[server(HandleSmartPlugToggle)]
pub async fn handle_smart_plug_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on},
        sqlx::PgPool,
    };

    let pool = use_context::<PgPool>().unwrap();
    let query = "
        UPDATE device
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

#[server(HandleSmartPowerStripToggle)]
pub async fn handle_smart_power_strip_toggle(
    state: bool,
    ip: String,
    child_id: String,
) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink::{
            tplink_turn_smart_strip_socket_off, tplink_turn_smart_strip_socket_on,
        },
        sqlx::PgPool,
    };

    let pool = use_context::<PgPool>().unwrap();
    let query = "
        UPDATE device
        SET power_state = $1
        WHERE ip = $2
    ";
    sqlx::query(query)
        .bind(if state { 1 } else { 0 })
        .bind(&ip)
        .execute(&pool)
        .await?;
    if state {
        tplink_turn_smart_strip_socket_on(&ip, &child_id).await;
    } else {
        tplink_turn_smart_strip_socket_off(&ip, &child_id).await;
    }
    Ok(())
}

#[server(HandleSmartLightToggle)]
pub async fn handle_smart_light_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use {crate::integrations::tplink::tplink_turn_light_on_off, sqlx::PgPool};

    let state = if state { 1 } else { 0 };

    let pool = use_context::<PgPool>().unwrap();
    let query = "
        UPDATE device
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

#[server(HandleSmartDimmerBrightness)]
pub async fn handle_smart_dimmer_brightness(
    brightness: u8,
    ip: String,
) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_set_dimmer_brightness;
    tplink_set_dimmer_brightness(&ip, &brightness).await;
    Ok(())
}

#[server(HandleSmartLightSaturation)]
pub async fn handle_smart_light_hsl(ip: String, color: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_set_light_hsl;
    tplink_set_light_hsl(&ip, color).await;
    Ok(())
}
