use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::US::Eastern;
use reqwest::{self, Method};
use std::collections::HashMap;
use std::str;

use crate::types::{SocketTicketRes, VideoSearchRes};

static CLIENT_API_BASE_URL: &str = "https://api.ring.com/clients_api/";
static DEVICE_API_BASE_URL: &str = "https://api.ring.com/devices/v1/";
static COMMANDS_API_BASE_URL: &str = "https://api.ring.com/commands/v1/";
static SNAPSHOTS_API_BASE_URL: &str = "https://app-snaps.ring.com/snapshots/";
static APP_API_BASE_URL: &str = "https://app.ring.com/api/v1/";
static OAUTH_API_BASE_URL: &str = "https://oauth.ring.com/oauth/token";

pub struct RingRestClient {
    pub refresh_token: String,
    pub hardware_id: String,
    pub auth_token: String,
}

impl RingRestClient {
    pub fn new(refresh_token: String, hardware_id: String, auth_token: String) -> Self {
        return Self {
            refresh_token,
            hardware_id,
            auth_token,
        };
    }

    pub async fn request_auth_token(&self) -> String {
        let mut map = HashMap::new();
        map.insert("client_id", "ring_official_android");
        map.insert("scope", "client");
        map.insert("grant_type", "refresh_token");
        map.insert("refresh_token", &self.refresh_token);

        let client = reqwest::Client::new();
        let res = client
            .post(OAUTH_API_BASE_URL)
            .json(&map)
            .header("2fa-support", "true")
            .header("2fa-code", "")
            .header("User-Agent", "android:com.ringapp")
            .header("hardware_id", &self.hardware_id)
            .send()
            .await
            .unwrap();
        res.text().await.unwrap()
    }

    pub async fn request(&self, path: &str, method: Method) -> String {
        let mut map = HashMap::new();
        map.insert("client_id", "ring_official_android");

        let client = reqwest::Client::new();
        let auth_value = format!("{}{}", "Bearer ", &self.auth_token);

        let res = client
            .request(method, path)
            .json(&map)
            .header("authorization", auth_value)
            .header("hardware_id", &self.hardware_id)
            .header("User-Agent", "android:com.ringapp")
            .send()
            .await
            .unwrap();

        println!("{}", res.status());
        return res.text().await.unwrap();
    }

    pub async fn get_locations(&self) -> String {
        self.request(&format!("{DEVICE_API_BASE_URL}locations"), Method::GET)
            .await
    }

    pub async fn get_devices(&self) -> String {
        self.request(&format!("{CLIENT_API_BASE_URL}ring_devices"), Method::GET)
            .await
    }

    pub async fn get_ws_url(&self) -> String {
        let socket_ticket_res = self
            .request(
                &format!("{APP_API_BASE_URL}clap/ticket/request/signalsocket"),
                Method::POST,
            )
            .await;

        let socket_ticket = serde_json::from_str::<SocketTicketRes>(&socket_ticket_res)
            .expect(&format!("locations_res: {socket_ticket_res}"));

        format!("wss://api.prod.signalling.ring.devices.a2z.com:443/ws?api_version=4.0&auth_type=ring_solutions&client_id=ring_site-3333&token={}", &socket_ticket.ticket)
    }

    pub async fn get_camera_snapshot(&self, id: &str) -> (String, axum::body::Bytes) {
        let mut map = HashMap::new();
        map.insert("client_id", "ring_official_android");

        let client = reqwest::Client::new();

        let res = client
            .get(&format!("{SNAPSHOTS_API_BASE_URL}next/{id}"))
            .json(&map)
            .bearer_auth(&self.auth_token)
            .header("hardware_id", &self.hardware_id)
            .header("User-Agent", "android:com.ringapp")
            .send()
            .await
            .unwrap();

        let time_ms = res
            .headers()
            .get("x-time-millis")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();

        let time = DateTime::<Utc>::from_timestamp((time_ms / 1000.) as i64, 0)
            .unwrap()
            .to_string();

        println!("{}", res.status());
        (time, res.bytes().await.unwrap())
    }

    pub async fn get_recordings(&self, id: &u64) -> String {
        let date_from: i64 = 1699506000000;
        let date_to: i64 = 1699592399999;

        let recordings_url = &format!(
            "{CLIENT_API_BASE_URL}video_search/history?doorbot_id={id}&date_from={date_from}&date_to={date_to}&order=asc&api_version=11"
        );

        self.request(&recordings_url, Method::GET).await
    }

    pub async fn get_camera_events(&self, location_id: &str, device_id: &u64) -> String {
        let camera_events_url =
            &format!("{CLIENT_API_BASE_URL}/locations/{location_id}/devices/{device_id}/events",);
        self.request(camera_events_url, Method::GET).await
    }
}

pub fn camera_recordings_list(recordings: VideoSearchRes) -> String {
    let mut element_text = String::from("<ul>");

    for recording in recordings.video_search {
        // Convert the created_at timestamp to Eastern Time
        let created_at_utc = Utc
            .timestamp_millis_opt(recording.created_at as i64)
            .unwrap();
        let created_at_eastern = created_at_utc.with_timezone(&Eastern);

        // Format the timestamp nicely, e.g., "April 5, 2023, 07:30 PM"
        let formatted_time = created_at_eastern.format("%B %e, %Y, %I:%M %p").to_string();

        element_text = format!(
            "{}\n<li><a href=\"{}\" target=\"_blank\">{}</a></li>",
            element_text, recording.hq_url, formatted_time
        );
    }

    element_text.push_str("</ul>");
    element_text
}
