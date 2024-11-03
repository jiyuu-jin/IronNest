use leptos::*;

#[server(HandleSmartPlugToggle)]
pub async fn handle_smart_plug_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on};
    if state {
        tplink_turn_plug_on(&ip).await;
    } else {
        tplink_turn_plug_off(&ip).await;
    }
    Ok(())
}

#[server(HandleSmartLightToggle)]
pub async fn handle_smart_light_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_turn_light_on_off;
    tplink_turn_light_on_off(&ip, if state { 1 } else { 0 }).await;
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
