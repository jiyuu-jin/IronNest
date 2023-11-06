use reqwest;
use std::collections::HashMap;
use std::str;

static CLIENT_API_BASE_URL: &str = "https://api.ring.com/clients_api/";
static DEVICE_API_BASE_URL: &str = "https://api.ring.com/devices/v1/";
static COMMANDS_API_BASE_URL: &str = "https://api.ring.com/commands/v1/";
static APP_API_BASE_URL: &str = "https://app.ring.com/api/v1/";
static OAUTH_API_BASE_URL: &str ="https://oauth.ring.com/oauth/token";

pub struct RingRestClient {
  pub refresh_token: String,
  pub hardware_id: String,
  pub auth_token: String,
}

impl RingRestClient {
  pub fn new(refresh_token: String, hardware_id:String, auth_token: String) -> Self {
    return Self { refresh_token, hardware_id, auth_token }
  }

  pub async fn request_auth_token(&self) -> String {
    let mut map = HashMap::new();
    map.insert("client_id", "ring_official_android");
    map.insert("scope", "client");
    map.insert("grant_type", "refresh_token");
    map.insert("refresh_token", &self.refresh_token);

    let client = reqwest::Client::new();
    let res = client.post(OAUTH_API_BASE_URL)
      .json(&map)
      .header("2fa-support","true")
      .header("2fa-code", "")
      .header("User-Agent", "android:com.ringapp")
      .header("hardware_id", &self.hardware_id)
      .send()
      .await
      .unwrap();
    res.text().await.unwrap()
  }

  pub async fn request(&self, path: &str) -> String {
    let mut map = HashMap::new();
    map.insert("client_id", "ring_official_android");

    let client = reqwest::Client::new();
    let auth_value = format!("{}{}", "Bearer ", &self.auth_token);

    let res = client.get(path)
      .json(&map)
      .header("authorization", auth_value)
      .header("hardware_id",&self.hardware_id)
      .header("User-Agent","android:com.ringapp")
      .send()
      .await
      .unwrap();

    println!("{}", res.status());
    return res.text().await.unwrap()
  }

  pub async fn get_locations(&self) -> String {
    self.request(&format!("{}{}", DEVICE_API_BASE_URL, "locations")).await
  }

  pub async fn get_devices(&self) -> String {
    self.request(&format!("{}{}", CLIENT_API_BASE_URL, "ring_devices")).await
  }

  pub async fn get_socket_ticket(&self) -> String {
    self.request(&format!("{}{}", APP_API_BASE_URL, "clap/ticket/request/signalsocket")).await
  }
}
