use {futures::prelude::*, ssdp_client::SearchTarget, std::time::Duration};

pub async fn discover_roku() {
    let search_target = SearchTarget::RootDevice;
    let mut responses = ssdp_client::search(&search_target, Duration::from_secs(3), 2, None)
        .await
        .unwrap();

    while let Some(response) = responses.next().await {
        match response {
            Ok(resp) => {
                println!("Location: {}", resp.location());
                println!("USN: {}", resp.usn());
                println!("Server: {}", resp.server());
                println!("------------------------");
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
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
