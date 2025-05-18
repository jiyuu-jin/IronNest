use {
    crate::integrations::efuy::types::{ApiResponse, CountryDomainResponse},
    aes::{
        cipher::{generic_array::typenum::U16, BlockEncrypt, KeyInit},
        Aes256,
    },
    base64::Engine,
    chrono::Utc,
    elliptic_curve::{generic_array::GenericArray, sec1::ToEncodedPoint},
    hex::decode,
    http::{HeaderMap, HeaderValue},
    p256::{ecdh::EphemeralSecret, PublicKey},
    rand_core::OsRng,
    reqwest::Client,
    serde_json::json,
    std::{env, iter},
};

static API_URL: &str = "https://extend.eufylife.com";
static TIMEZONE: &str = "GMT+01:00";
static SERVER_PUBLIC_KEY:&str = "04c5c00c4f8d1197cc7c3167c52bf7acb054d722f0ef08dcd7e0883236e0d72a3868d9750cb47fa4619248f3d83f0f662671dadc6e2d31c2f41db0161651c7c076";
static SN: &str = "75814221ee75";
static OS_TYPE: &str = "android";

pub async fn get_country_url(client: &Client) -> String {
    let country_domain_str = client
        .get(format!("{API_URL}/domain/US"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let country_domain_str: CountryDomainResponse =
        serde_json::from_str(&country_domain_str).unwrap();
    let country_url = format!("https://{}", country_domain_str.data.domain);
    println!("Country URL: {country_url}");
    country_url
}

pub fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("App_version", HeaderValue::from_static("v4.6.0_1630"));
    headers.insert("Os_type", HeaderValue::from_static(OS_TYPE));
    headers.insert("Os_version", HeaderValue::from_static("31"));
    headers.insert("Phone_model", HeaderValue::from_static("ONEPLUS A3003"));
    headers.insert("Country", HeaderValue::from_static("US"));
    headers.insert("Language", HeaderValue::from_static("en"));
    headers.insert("Openudid", HeaderValue::from_static("5e4621b0152c0d00"));
    headers.insert("Net_type", HeaderValue::from_static("wifi"));
    headers.insert("Mnc", HeaderValue::from_static("02"));
    headers.insert("Sn", HeaderValue::from_static(SN));
    headers.insert("Model_type", HeaderValue::from_static("PHONE"));
    headers.insert("Timezone", HeaderValue::from_static(TIMEZONE));
    headers.insert("Cache-Control", HeaderValue::from_static("no-cache"));
    headers
}

pub async fn eufy_login() -> ApiResponse {
    let username = env::var("EUFY_USERNAME").expect("EUFY_USERNAME not found in environment");
    let password = env::var("EUFY_PASSWORD").expect("EUFY_PASSWORD not found in environment");

    let client = reqwest::Client::new();

    let country_domain_str = client
        .get(format!("{API_URL}/domain/US"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let country_domain_str: CountryDomainResponse =
        serde_json::from_str(&country_domain_str).unwrap();
    let country_url = format!("https://{}", country_domain_str.data.domain);
    println!("Country URL: {country_url}");

    let secret = EphemeralSecret::random(&mut OsRng);
    let public_key = PublicKey::from(&secret);
    let public_key_bytes = public_key.to_encoded_point(false).to_bytes();

    // Convert byte array to hex string for serialization
    let public_key_hex = hex::encode(public_key_bytes);

    // Decode the server's public key from hex
    let server_public_key_bytes =
        decode(SERVER_PUBLIC_KEY).expect("Invalid hex in server public key");
    let server_public_key =
        PublicKey::from_sec1_bytes(&server_public_key_bytes).expect("Invalid server public key");

    // Compute the shared secret
    let shared_secret = secret.diffie_hellman(&server_public_key);

    // Convert shared secret to a byte array (32 bytes)
    let shared_secret_bytes = shared_secret.raw_secret_bytes();

    // Convert GenericArray reference to a slice
    let key_slice: &[u8] = shared_secret_bytes.as_slice();

    let encrypted_password = encrypt_api_data(&password, key_slice).expect("Encryption failed");
    let request_body = &json!({
        "ab": "US",
        "client_secret_info": {
            "public_key": public_key_hex,
        },
        "enc": 0,
        "email": username,
        "password":  encrypted_password,
        "time_zone": TIMEZONE,
        "transaction": Utc::now().timestamp_millis(),
    });

    let headers = get_headers();

    let auth_res = client
        .post(format!("{country_url}/v2/passport/login_sec"))
        .json(&request_body)
        .headers(headers)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    println!("Auth res: {auth_res}");

    let auth = serde_json::from_str::<ApiResponse>(&auth_res).unwrap();
    println!("Got Eufy auth_token: {:?}", auth.data.auth_token);
    auth
}

pub async fn get_devices(auth_token: String) {
    let client = reqwest::Client::new();
    let country_url = get_country_url(&client).await;

    let mut headers = get_headers();
    println!("auth_token: {auth_token}");
    headers.insert("X-Auth-Token", HeaderValue::from_str(&auth_token).unwrap());

    let res = client
        .post(format!("{country_url}/v2/house/device_list"))
        .json(&json!({
            "device_sn": "",
            "num": 1000,
            "orderby": "",
            "page": 0,
            "station_sn": "",
            "time_zone": TIMEZONE,
            "transaction": Utc::now().timestamp_millis()
        }))
        .headers(headers)
        .send()
        .await
        .unwrap();
    println!("res: {res:?}");
    let device_res = res.text().await.unwrap();
    println!("device_res: {:?}", device_res);
}

fn encrypt_api_data(data: &str, key: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if key.len() != 32 {
        return Err("Key length must be 32 bytes for AES-256".into());
    }

    // PKCS7 padding
    let mut padded_data = data.as_bytes().to_vec();
    let padding_len = 16 - (padded_data.len() % 16);
    padded_data.extend(iter::repeat_n(padding_len as u8, padding_len));

    // Split the key into the AES key and the IV
    let aes_key = GenericArray::from_slice(key);
    let iv = GenericArray::<u8, U16>::clone_from_slice(&key[..16]);
    let cipher = Aes256::new(aes_key);

    // Encrypt block by block
    let mut encrypted_data = Vec::with_capacity(padded_data.len());
    let mut block_iv = iv;

    for chunk in padded_data.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);

        // XOR with IV
        for (b, iv_byte) in block.iter_mut().zip(block_iv.iter()) {
            *b ^= *iv_byte;
        }

        cipher.encrypt_block(&mut block);
        block_iv = block;
        encrypted_data.extend_from_slice(&block);
    }

    Ok(base64::engine::general_purpose::STANDARD.encode(&encrypted_data))
}
