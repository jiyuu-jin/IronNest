use axum::{extract::Path, http::StatusCode};

pub async fn roku_keypress_handler(
    Path((device_id, key)): Path<(i64, String)>,
) -> Result<String, StatusCode> {
    let roku_ip = if device_id == 1 {
        "192.168.0.220"
    } else {
        "10.0.0.217"
    };

    let roku_url = format!("http://{}:8060/keypress/{}", roku_ip, key);
    let client = reqwest::Client::new();

    match client.post(&roku_url).send().await {
        Ok(_) => Ok(format!("Key pressed: {}", key)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
