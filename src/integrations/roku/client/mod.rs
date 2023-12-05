use {
    super::types::{ActionApp, RokuDiscoverRes},
    futures::prelude::*,
    serde_xml_rs::from_str,
    ssdp_client::SearchTarget,
    std::time::Duration,
};

pub async fn discover_roku() -> Vec<RokuDiscoverRes> {
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

pub async fn get_roku_apps() -> String {
    let roku_url = "http://10.0.0.162:8060/query/apps";
    let client = reqwest::Client::new();

    client
        .post(roku_url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

pub async fn get_active_app() -> ActionApp {
    let roku_url = "http://192.168.0.220:8060/query/active-app";
    let client = reqwest::Client::new();

    let app_text = client
        .get(roku_url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    from_str(&app_text).unwrap()
}

pub async fn get_media_player() -> String {
    let roku_url = "http://192.168.0.220:8060/query/media-player";
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

pub async fn get_active_channel() -> String {
    let roku_url = "http://192.168.0.220:8060/query/tv-active-channel";
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
