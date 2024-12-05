use {
    super::types::TuyaDeviceRes,
    chrono::Utc,
    hmac::{Hmac, Mac},
    http::{HeaderMap, HeaderName, HeaderValue, Method},
    reqwest::Client,
    serde::{Deserialize, Serialize},
    sha2::{Digest, Sha256},
    std::{env, error::Error},
    url::Url,
};

static TUYA_API_URL: &str = "https://openapi.tuyaus.com";

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct State {
    pub refresh_token: String,
    pub hardware_id: String,
    pub auth_token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TuyaAuthRes {
    pub result: TuyaAuthValues,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TuyaAuthValues {
    pub access_token: String,
    pub refresh_token: String,
    pub uid: String,
}

#[derive(Debug)]
pub struct TuyaRestClient {
    pub state: State,
}

pub async fn get_refresh_token() -> Result<TuyaAuthRes, Box<dyn Error>> {
    let res = request("/v1.0/token?grant_type=1", "").await;
    let tuya_auth: TuyaAuthRes = serde_json::from_str(&res)?;
    println!("{:?}", tuya_auth);
    Ok(tuya_auth)
}

pub async fn get_devices(user_id: &str, token: &str) -> Result<TuyaDeviceRes, Box<dyn Error>> {
    let res = request(&format!("/v1.0/users/{user_id}/devices"), token).await;
    let tuya_devices: TuyaDeviceRes = serde_json::from_str(&res)?;
    Ok(tuya_devices)
}

pub async fn get_user_id(device_id: &str, token: &str) -> String {
    request(&format!("/v1.0/devices/{device_id}"), token).await
}

pub async fn request(path: &str, token: &str) -> String {
    let tuya_client_id =
        env::var("TUYA_CLIENT_ID").expect("TUYA_CLIENT_ID not found in environment");
    let tuya_api_key = env::var("TUYA_API_KEY").expect("TUYA_API_KEY not found in environment");

    let api_url = TUYA_API_URL.parse::<Url>().unwrap().join(path).unwrap();

    let method = Method::GET;
    let body: Option<String> = None;

    let content_type: Option<String> = None;
    let signed_headers = if let Some(content_type) = content_type {
        vec![(
            "content-type",
            HeaderValue::from_str(&content_type).unwrap(),
        )]
    } else {
        vec![]
    };
    let signature_header_value = HeaderValue::from_str(
        &signed_headers
            .iter()
            .map(|(name, _)| name.to_owned())
            .collect::<Vec<_>>()
            .join(":"),
    )
    .unwrap();

    let mut payload = tuya_client_id.clone();
    let secret_or_access_token_header = if !token.is_empty() {
        payload.push_str(token);
        [("access_token", HeaderValue::from_str(token).unwrap())]
    } else {
        [("secret", HeaderValue::from_str(&tuya_api_key).unwrap())]
    };
    let now = Utc::now().timestamp_millis().to_string();
    payload.push_str(&now);
    payload.push_str(&format!("{method}\n"));
    payload.push_str(&format!(
        "{:x}\n",
        Sha256::digest(
            #[allow(clippy::unnecessary_literal_unwrap)]
            body.unwrap_or_else(|| "".to_owned())
        )
    ));
    for (name, value) in signed_headers.iter() {
        payload.push_str(&format!("{name}:{}\n", value.to_str().unwrap()));
    }
    payload.push('\n');
    payload.push_str(path);
    let mut hmac = Hmac::<Sha256>::new_from_slice(tuya_api_key.as_bytes()).unwrap();
    hmac.update(payload.as_bytes());
    let signature = hex::encode_upper(hmac.finalize().into_bytes());

    let headers = HeaderMap::from_iter(
        signed_headers
            .into_iter()
            .chain([("signature-headers", signature_header_value)])
            .chain(secret_or_access_token_header)
            .chain([
                ("client_id", HeaderValue::from_str(&tuya_client_id).unwrap()),
                ("sign", HeaderValue::from_str(&signature).unwrap()),
                ("t", HeaderValue::from_str(&now).unwrap()),
                ("sign_method", HeaderValue::from_static("HMAC-SHA256")),
                ("mode", HeaderValue::from_static("cors")),
                ("lang", HeaderValue::from_static("en")),
            ])
            .map(|(name, value)| (HeaderName::from_static(name), value)),
    );

    Client::new()
        .request(method, api_url)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap()
}

pub async fn discover_tuya_devices() -> Result<(), Box<dyn std::error::Error>> {
    let port = 6666;

    // Bind UDP socket to listen for unencrypted Tuya device broadcasts
    let socket = tokio::net::UdpSocket::bind(("0.0.0.0", port)).await?;
    println!("Listening for Tuya devices on port {}", port);

    let mut buf = [0u8; 2048];

    loop {
        // Receive incoming UDP messages
        match socket.recv_from(&mut buf).await {
            Ok((num_bytes, src_addr)) => {
                // Exclude loopback messages
                if !is_loopback(&src_addr) {
                    println!(
                        "Received {} bytes from {}: {:?}",
                        num_bytes,
                        src_addr,
                        &buf[..num_bytes]
                    );
                }
            }
            Err(e) => {
                println!("Error receiving UDP message: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Check if a given address is a loopback address.
fn is_loopback(addr: &std::net::SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(ip) => ip.is_loopback(),
        std::net::IpAddr::V6(ip) => ip.is_loopback(),
    }
}
