use super::types::RokuDiscoverRes;

use {futures::prelude::*, ssdp_client::SearchTarget, std::time::Duration};

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
