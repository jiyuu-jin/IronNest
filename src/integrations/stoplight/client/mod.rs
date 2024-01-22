use {
    async_nats::jetstream,
    log::info,
    serde::{Deserialize, Serialize},
    std::io::Error,
};

const STOPLIGHT_BUCKET: &str = "stoplight";
const STOPLIGHT_SUBJECT: &str = "stoplight";

#[derive(Debug, Serialize, Deserialize)]
pub struct Stoplight {
    pub red: bool,
    pub yellow: bool,
    pub green: bool,
}

pub async fn toggle_stoplight() -> Result<&'static str, Error> {
    info!("sending command to stoplight");
    let nats = async_nats::ConnectOptions::with_credentials_file("default.creds")
        .await
        .unwrap()
        .require_tls(true)
        .connect("connect.ngs.global")
        .await
        .unwrap();

    let js = jetstream::new(nats);
    let kv = js.get_key_value(STOPLIGHT_BUCKET).await.unwrap();

    let value = serde_json::from_slice::<Stoplight>(
        &kv.get(STOPLIGHT_SUBJECT)
            .await
            .unwrap()
            .ok_or(Stoplight {
                red: false,
                yellow: false,
                green: false,
            })
            .unwrap(),
    )
    .unwrap();

    kv.put(
        STOPLIGHT_SUBJECT,
        serde_json::to_string(&Stoplight {
            red: true,
            yellow: !value.yellow,
            green: !value.green,
        })
        .unwrap()
        .into(),
    )
    .await
    .unwrap();

    Ok("success")
}
