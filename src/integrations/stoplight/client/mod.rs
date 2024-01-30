use {
    async_nats::jetstream,
    log::info,
    serde::{Deserialize, Serialize},
    std::error::Error,
};

const STOPLIGHT_BUCKET: &str = "stoplight";
const STOPLIGHT_SUBJECT: &str = "stoplight";

#[derive(Debug, Serialize, Deserialize)]
pub struct Stoplight {
    pub red: bool,
    pub yellow: bool,
    pub green: bool,
}

pub async fn stoplight_get_state() -> Result<Stoplight, Box<dyn Error>> {
    let nats = async_nats::ConnectOptions::with_credentials_file("default.creds")
        .await?
        .require_tls(true)
        .connect("connect.ngs.global")
        .await?;

    let js = jetstream::new(nats);
    let kv = js.get_key_value(STOPLIGHT_BUCKET).await?;

    let stoplight_value = kv.get(STOPLIGHT_SUBJECT).await.unwrap().unwrap();
    let value: Stoplight = serde_json::from_slice(&stoplight_value).unwrap();
    Ok(value)
}

pub async fn toggle_stoplight(color: &str) -> Result<&'static str, Box<dyn Error>> {
    info!("sending command to stoplight");

    let nats = async_nats::ConnectOptions::with_credentials_file("default.creds")
        .await?
        .require_tls(true)
        .connect("connect.ngs.global")
        .await?;

    let js = jetstream::new(nats);
    let kv = js.get_key_value(STOPLIGHT_BUCKET).await?;
    let mut value = stoplight_get_state().await?;

    match color {
        "red" => value.red = !value.red,
        "yellow" => value.yellow = !value.yellow,
        "green" => value.green = !value.green,
        _ => info!("not found"),
    }

    kv.put(STOPLIGHT_SUBJECT, serde_json::to_string(&value)?.into())
        .await?;

    Ok("success")
}
