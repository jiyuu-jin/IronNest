use {
    crate::types::{
        AuthResponse, CameraEventsRes, DevicesRes, LocationsRes, SocketTicketRes, VideoSearchRes,
    },
    chrono::{DateTime, TimeZone, Utc},
    chrono_tz::US::Eastern,
    log::{info, warn},
    reqwest::{self, Client, Method},
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, fs::File, str, sync::RwLock},
    uuid::Uuid,
};

static CLIENT_API_BASE_URL: &str = "https://api.ring.com/clients_api/";
static DEVICE_API_BASE_URL: &str = "https://api.ring.com/devices/v1/";
static _COMMANDS_API_BASE_URL: &str = "https://api.ring.com/commands/v1/";
static SNAPSHOTS_API_BASE_URL: &str = "https://app-snaps.ring.com/snapshots/";
static APP_API_BASE_URL: &str = "https://app.ring.com/api/v1/";
static OAUTH_API_BASE_URL: &str = "https://oauth.ring.com/oauth/token";

#[derive(Debug, Serialize, Deserialize)]
struct State {
    pub refresh_token: String,
    pub hardware_id: String,
    pub auth_token: String,
}

#[derive(Debug)]
pub struct RingRestClient {
    state: RwLock<State>,
    pub client: Client,
}

const STATE_FILE_NAME: &str = "state.json";

fn read_state_from_file() -> std::io::Result<State> {
    let state_file = File::open(STATE_FILE_NAME);
    let result = match state_file {
        Ok(file) => {
            let state = serde_json::from_reader(file);
            match state {
                Ok(state) => Some(state),
                Err(e) => {
                    warn!("Error reading from state file, resetting to empty state: {e}");
                    None
                }
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                info!("State file not found, defaulting to empty state");
                None
            } else {
                return Err(e);
            }
        }
    };

    Ok(result.unwrap_or_else(|| State {
        refresh_token: "".to_owned(),
        hardware_id: Uuid::new_v4().to_string(),
        auth_token: "".to_owned(),
    }))
}

fn write_state_to_file(state: &State) -> std::io::Result<()> {
    let state_file = File::create(STATE_FILE_NAME)?;
    serde_json::to_writer(state_file, state)?;
    Ok(())
}

impl RingRestClient {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            state: RwLock::new(read_state_from_file().unwrap()),
            client: reqwest::Client::new(),
        }
    }

    pub async fn request_auth_token(&self, username: &str, password: &str, two_fa: &str) -> String {
        let mut request_body =
            HashMap::from([("client_id", "ring_official_android"), ("scope", "client")]);

        let res = {
            let (refresh_token, hardware_id) = {
                let state = self.state.read().unwrap();
                (state.refresh_token.clone(), state.hardware_id.clone())
            };

            if !username.is_empty() && !password.is_empty() {
                request_body.insert("grant_type", "password");
                request_body.insert("username", username);
                request_body.insert("password", password);
            } else {
                request_body.insert("refresh_token", &refresh_token);
                request_body.insert("grant_type", "refresh_token");
            };

            self.client
                .post(OAUTH_API_BASE_URL)
                .json(&request_body)
                .header("2fa-support", "true")
                .header("2fa-code", two_fa)
                .header("User-Agent", "android:com.ringapp")
                .header("hardware_id", &hardware_id)
                .send()
                .await
                .unwrap()
        };
        if res.status().is_success() {
            let text = res.text().await.unwrap();
            println!("response: {text}");

            let auth_res = serde_json::from_str::<AuthResponse>(&text)
                .unwrap_or_else(|_| panic!("error requesting: {text}"));

            let mut state = self.state.write().unwrap();
            state.auth_token = auth_res.access_token;
            state.refresh_token = auth_res.refresh_token;
            write_state_to_file(&state).unwrap();
            "Login successful".to_string()
        } else {
            res.text().await.unwrap()
        }
    }

    pub async fn request(&self, path: &str, method: Method) -> String {
        let request_body = HashMap::from([("client_id", "ring_official_android")]);

        let (auth_token, hardware_id) = {
            let state = self.state.read().unwrap();
            (state.auth_token.clone(), state.hardware_id.clone())
        };
        let auth_value = format!("{}{}", "Bearer ", &auth_token);

        let res = self
            .client
            .request(method, path)
            .json(&request_body)
            .header("authorization", auth_value)
            .header("hardware_id", &hardware_id)
            .header("User-Agent", "android:com.ringapp")
            .send()
            .await
            .unwrap();

        println!("{}", res.status());
        res.text().await.unwrap()
    }

    pub async fn get_locations(&self) -> LocationsRes {
        let res = self
            .request(&format!("{DEVICE_API_BASE_URL}locations"), Method::GET)
            .await;
        serde_json::from_str::<LocationsRes>(&res)
            .unwrap_or_else(|_| panic!("locations_res: {res}"))
    }

    pub async fn get_devices(&self) -> DevicesRes {
        let res = self
            .request(&format!("{CLIENT_API_BASE_URL}ring_devices"), Method::GET)
            .await;
        serde_json::from_str::<DevicesRes>(&res).unwrap_or_else(|_| panic!("devices_res: {res}"))
    }

    pub async fn get_camera_events(&self, location_id: &str, device_id: &u64) -> CameraEventsRes {
        let camera_events_url =
            &format!("{CLIENT_API_BASE_URL}/locations/{location_id}/devices/{device_id}/events",);

        let res = self.request(camera_events_url, Method::GET).await;
        serde_json::from_str::<CameraEventsRes>(&res)
            .unwrap_or_else(|_| panic!("camera_event_res: {res}"))
    }

    pub async fn get_ws_url(&self) -> String {
        let socket_ticket_res = self
            .request(
                &format!("{APP_API_BASE_URL}clap/ticket/request/signalsocket"),
                Method::POST,
            )
            .await;

        let socket_ticket = serde_json::from_str::<SocketTicketRes>(&socket_ticket_res)
            .unwrap_or_else(|_| panic!("locations_res: {socket_ticket_res}"));

        format!("wss://api.prod.signalling.ring.devices.a2z.com:443/ws?api_version=4.0&auth_type=ring_solutions&client_id=ring_site-3333&token={}", &socket_ticket.ticket)
    }

    pub async fn get_camera_snapshot(&self, id: &str) -> (String, bytes::Bytes) {
        let request_body = HashMap::from([("client_id", "ring_official_android")]);

        let (auth_token, hardware_id) = {
            let state = self.state.read().unwrap();
            (state.auth_token.clone(), state.hardware_id.clone())
        };
        let res = self
            .client
            .get(&format!("{SNAPSHOTS_API_BASE_URL}next/{id}"))
            .json(&request_body)
            .bearer_auth(&auth_token)
            .header("hardware_id", &hardware_id)
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

    pub async fn get_recordings(&self, id: &u64) -> VideoSearchRes {
        let now = Utc::now();
        let date_from = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .timestamp_millis();
        let date_to = now
            .date_naive()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .timestamp_millis();

        let recordings_url = &format!(
            "{CLIENT_API_BASE_URL}video_search/history?doorbot_id={id}&date_from={date_from}&date_to={date_to}&order=asc&api_version=11"
        );

        let res = self.request(recordings_url, Method::GET).await;
        serde_json::from_str::<VideoSearchRes>(&res)
            .unwrap_or_else(|_| panic!("camera_event_res: {res}"))
    }

    pub async fn subscribe_to_motion_events(&self, device_id: &u64) {
        let recordings_url = &format!("{CLIENT_API_BASE_URL}devices/{device_id}/motions_subscribe");
        println!("{recordings_url}");
        let res = self.request(recordings_url, Method::POST).await;
        println!("subscribe motion events: {res}");
    }
}

pub fn camera_recordings_list(recordings: VideoSearchRes) -> String {
    "<ul>"
        .chars()
        .chain(recordings.video_search.iter().flat_map(|recording| {
            // Convert the created_at timestamp to Eastern Time
            let created_at_utc = Utc
                .timestamp_millis_opt(recording.created_at as i64)
                .unwrap();
            let created_at_eastern = created_at_utc.with_timezone(&Eastern);

            // Format the timestamp nicely, e.g., "April 5, 2023, 07:30 PM"
            let formatted_time = created_at_eastern.format("%B %e, %Y, %I:%M %p").to_string();

            format!(
                "\n<li><a href=\"{}\" target=\"_blank\">{}</a></li>",
                recording.hq_url, formatted_time
            )
            .chars()
            .collect::<Vec<_>>()
        }))
        .chain("</ul>".chars())
        .collect::<String>()
}
