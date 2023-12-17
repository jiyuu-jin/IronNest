use {
    super::types::{ActionApp, RokuDiscoverRes},
    futures::prelude::*,
    serde_json::json,
    serde_xml_rs::from_str,
    ssdp_client::SearchTarget,
    std::time::Duration,
    tokio_tungstenite::{connect_async, tungstenite::protocol::Message},
    url::Url,
};

pub async fn roku_discover() -> Vec<RokuDiscoverRes> {
    let search_target = SearchTarget::RootDevice;
    let mut responses = ssdp_client::search(&search_target, Duration::from_secs(3), 2, None)
        .await
        .unwrap();
    let mut devices = Vec::with_capacity(20);

    while let Some(response) = responses.next().await {
        match response {
            Ok(resp) => {
                if resp.server().to_string().contains("Roku") {
                    devices.push(RokuDiscoverRes {
                        location: resp.location().to_string(),
                        usn: resp.usn().to_string(),
                        server: resp.server().to_string(),
                    });
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
    devices
}

pub async fn roku_get_apps() -> String {
    get("query/apps").await
}

pub async fn roku_get_media_player() -> String {
    get("query/media-player").await
}

pub async fn roku_get_active_app() -> ActionApp {
    let app_text = get("query/active-app").await;
    from_str(&app_text).unwrap()
}

pub async fn roku_send_keypress(key: &str) -> serde_json::Value {
    post(format!("keypress/{key}").as_str()).await
}

pub async fn roku_search(query: &str) -> serde_json::Value {
    post(format!("search/browse?{query}={query}&matchAny=true").as_str()).await
}

pub async fn roku_launch_app(app_id: &str) -> serde_json::Value {
    post(format!("launch/{app_id}").as_str()).await
}

pub async fn post(query: &str) -> serde_json::Value {
    let roku_url = format!("http://10.0.0.162:8060/{query}");
    let client = reqwest::Client::new();

    match client.post(&roku_url).send().await {
        Ok(data) => println!("input: {:?}", data),
        Err(err) => println!("Error! {err}"),
    };

    json!({
        "success": true,
    })
}

pub async fn get(query: &str) -> String {
    let roku_url = format!("http://10.0.0.162:8060/{query}");
    let client = reqwest::Client::new();

    client
        .get(roku_url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

pub async fn roku_ws() {
    // Parse the URL for the WebSocket connection
    let url = Url::parse("ws://192.168.0.220:8060").unwrap();

    // Establish a connection
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    // Split the stream into a sender and receiver
    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text("Hello WebSocket".into()))
        .await
        .unwrap();

    read.for_each(|message| async {
        let message = message.unwrap();
        println!("Received a message: {:?}", message);
    })
    .await;
}
