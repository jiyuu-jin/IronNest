mod options;
use {clap::Parser, options::Options, reqwest::Url};

#[tokio::main]
async fn main() {
    let options = Options::parse();
    println!("{:?}", options);

    let server_url = options
        .server_url
        .unwrap_or_else(|| Url::parse("http://localhost:3000").unwrap());

    let client = reqwest::Client::new();

    match options.operation {
        options::Operation::UploadFile {
            file_path,
            mish_state_name,
            path,
        } => {
            let data = tokio::fs::read(&file_path).await.unwrap();
            let req = if file_path.ends_with(".json") {
                let json = serde_json::from_slice::<serde_json::Value>(&data).unwrap();
                client
                    .post(server_url.join("/api/mish/blob.dag-json").unwrap())
                    .json(&json)
            } else {
                client
                    .post(server_url.join("/api/mish/blob.raw").unwrap())
                    .body(data)
                // .header("Content-Type", "application/octet-stream")
            };
            let result = req.send().await.unwrap();
            let cid = if result.status().is_success() {
                let cid = result.json::<String>().await.unwrap();
                println!("Uploaded file to Mish: {cid}");
                cid
            } else {
                panic!(
                    "Failed to upload file to Mish: {}, {:?}",
                    result.status(),
                    result.text().await
                );
            };

            let json = serde_json::json!({
                "mish_state_name": mish_state_name,
                "path": path,
                "content": serde_json::json!({"/": cid}),
            });

            let req = client
                .post(server_url.join("/api/mish/state").unwrap())
                .json(&json);
            let result = req.send().await.unwrap();
            if result.status().is_success() {
                println!("Updated Mish state: {}", result.text().await.unwrap());
            } else {
                panic!(
                    "Failed to update Mish state: {}, {:?}",
                    result.status(),
                    result.text().await
                );
            }
        }
    }
}
