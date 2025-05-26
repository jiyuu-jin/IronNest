mod options;
use {clap::Parser, options::Options};

#[tokio::main]
async fn main() {
    let options = Options::parse();
    println!("{:?}", options);

    match options.operation {
        options::Operation::UploadFile { path } => {
            let data = tokio::fs::read(&path).await.unwrap();
            let client = reqwest::Client::new();
            let req = if path.ends_with(".json") {
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
            if result.status().is_success() {
                let cid = result.json::<String>().await.unwrap();
                println!("Uploaded file to Mish: {cid}");
            } else {
                println!("Failed to upload file to Mish: {}", result.status());
                println!("{}", result.text().await.unwrap());
            }
        }
    }
}
