use leptos::prelude::*;

#[server(HandleRokuTvToggle)]
pub async fn handle_roku_tv_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::roku::roku_send_keypress;
    if state {
        roku_send_keypress(&ip, "PowerOff").await;
    } else {
        roku_send_keypress(&ip, "PowerOn").await;
    }
    Ok(())
}
