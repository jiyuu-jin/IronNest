mod options;
use {clap::Parser, options::Options};

#[tokio::main]
async fn main() {
    let options = Options::parse();
    println!("{:?}", options);

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
                    .post("http://localhost:3000/api/mish/blob.dag-json")
                    .json(&json)
            } else {
                client
                    .post("http://localhost:3000/api/mish/blob.raw")
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
                .post("http://localhost:3000/api/mish/state")
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
