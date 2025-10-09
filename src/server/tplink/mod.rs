use leptos::prelude::*;

#[server(HandleSmartPlugToggle)]
pub async fn handle_smart_plug_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink_bridge::get_tplink_bridge,
        sqlx::PgPool,
        tplink::Device,
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

    // Use the new device-based API
    let bridge = get_tplink_bridge();
    if let Some(Device::SmartPlug(plug)) = bridge.get_device(&ip).await {
        if state {
            plug.turn_on().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        } else {
            plug.turn_off().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        }
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
        crate::integrations::tplink_bridge::get_tplink_bridge,
        sqlx::PgPool,
        tplink::Device,
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

    let bridge = get_tplink_bridge();
    if let Some(Device::PowerStrip(strip)) = bridge.get_device(&ip).await {
        if state {
            strip.turn_socket_on(&child_id).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        } else {
            strip.turn_socket_off(&child_id).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        }
    }
    Ok(())
}

#[server(HandleSmartLightToggle)]
pub async fn handle_smart_light_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink_bridge::get_tplink_bridge,
        sqlx::PgPool,
        tplink::Device,
    };

    let pool = use_context::<PgPool>().unwrap();
    let query = "
        UPDATE device
        SET power_state = $1
        WHERE ip = $2
    ";
    println!("state: {state}");
    println!("ip: {ip}");
    sqlx::query(query)
        .bind(if state { 1 } else { 0 })
        .bind(&ip)
        .execute(&pool)
        .await?;

    let bridge = get_tplink_bridge();
    if let Some(Device::SmartLight(light)) = bridge.get_device(&ip).await {
        light.turn_on_off(state).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

#[server(HandleSmartLightBrightness)]
pub async fn handle_smart_light_brightness(
    brightness: u8,
    ip: String,
) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink_bridge::get_tplink_bridge,
        tplink::Device,
    };

    let bridge = get_tplink_bridge();
    if let Some(Device::SmartLight(light)) = bridge.get_device(&ip).await {
        light.set_brightness(brightness).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

#[server(HandleSmartDimmerBrightness)]
pub async fn handle_smart_dimmer_brightness(
    brightness: u8,
    ip: String,
) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink_bridge::get_tplink_bridge,
        tplink::Device,
    };

    let bridge = get_tplink_bridge();
    if let Some(Device::SmartDimmer(dimmer)) = bridge.get_device(&ip).await {
        dimmer.set_brightness(brightness).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

#[server(HandleSmartLightSaturation)]
pub async fn handle_smart_light_hsl(ip: String, color: String) -> Result<(), ServerFnError> {
    use {
        crate::integrations::tplink_bridge::get_tplink_bridge,
        tplink::Device,
    };

    let bridge = get_tplink_bridge();
    if let Some(Device::SmartLight(light)) = bridge.get_device(&ip).await {
        light.set_color(&color).await.map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}
