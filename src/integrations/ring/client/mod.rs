use {
    super::types::{
        AuthResponse, CameraEventsRes, DevicesRes, Doorbot, LocationsRes, RingCamera,
        RingCameraSnapshot, SocketTicketRes, VideoSearchRes,
    },
    crate::integrations::iron_nest::{get_auth_from_db, insert_auth, types::AuthState},
    base64::{engine::general_purpose::STANDARD as base64, Engine},
    chrono::{DateTime, Duration, Local, TimeZone, Utc},
    chrono_tz::US::Eastern,
    http::{header::ToStrError, StatusCode},
    log::{error, info},
    reqwest::{self, Client, Method, Response},
    serde::de::DeserializeOwned,
    sqlx::{Pool, Sqlite},
    std::{collections::HashMap, num::ParseFloatError, str, sync::Arc},
    uuid::Uuid,
};

static CLIENT_API_BASE_URL: &str = "https://api.ring.com/clients_api/";
static DEVICE_API_BASE_URL: &str = "https://api.ring.com/devices/v1/";
static _COMMANDS_API_BASE_URL: &str = "https://api.ring.com/commands/v1/";
static SNAPSHOTS_API_BASE_URL: &str = "https://app-snaps.ring.com/snapshots/";
static APP_API_BASE_URL: &str = "https://app.ring.com/api/v1/";
static OAUTH_API_BASE_URL: &str = "https://oauth.ring.com/oauth/token";

#[derive(Debug)]
pub struct RingRestClient {
    state: AuthState,
    pub client: Client,
    pool: Arc<Pool<Sqlite>>,
}

pub fn camera_recordings_list(recordings: VideoSearchRes) -> String {
    "<ul>"
        .chars()
        .chain(recordings.video_search.iter().flat_map(|recording| {
            let created_at_utc = Utc.timestamp_millis_opt(recording.created_at).unwrap();
            let created_at_eastern = created_at_utc.with_timezone(&Eastern);
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

#[derive(Debug, thiserror::Error)]
pub enum RingRestClientInternalError {
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Unexpected response code: {0} {1:?}")]
    UnexpectedResponseCode(StatusCode, reqwest::Result<String>),

    #[error("Ring snapshot API response headers missing x-time-millis header")]
    HeadersMissingXTimeMillis,

    #[error("Ring snapshot API response x-time-millis header cannot be converted to string: {0}")]
    XTimeMillisHeaderNotString(ToStrError),

    #[error("Ring snapshot API response x-time-millis header cannot be parsed: {0}")]
    XTimeMillisHeaderParseError(ParseFloatError),
}

#[derive(Debug, thiserror::Error)]
pub enum RingRestClientError {
    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal error: {0}")]
    InternalError(#[from] RingRestClientInternalError),
}

impl RingRestClient {
    #[allow(clippy::new_without_default)]
    pub async fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        let auth_db_pool = pool.clone();
        Self {
            pool,
            state: get_auth_from_db(auth_db_pool, "ring").await,
            client: reqwest::Client::new(),
        }
    }

    pub async fn request_auth_token(&self, username: &str, password: &str, two_fa: &str) -> String {
        let mut request_body =
            HashMap::from([("client_id", "ring_official_android"), ("scope", "client")]);

        let res = {
            if !username.is_empty() && !password.is_empty() {
                request_body.insert("grant_type", "password");
                request_body.insert("username", username);
                request_body.insert("password", password);
            } else {
                request_body.insert("refresh_token", &self.state.refresh_token);
                request_body.insert("grant_type", "refresh_token");
            };

            self.client
                .post(OAUTH_API_BASE_URL)
                .json(&request_body)
                .header("2fa-support", "true")
                .header("2fa-code", two_fa)
                .header("User-Agent", "android:com.ringapp")
                .header("hardware_id", &self.state.hardware_id)
                .send()
                .await
                .unwrap()
        };
        if res.status().is_success() {
            let text = res.text().await.unwrap();
            println!("response: {text}");

            let auth_res = serde_json::from_str::<AuthResponse>(&text)
                .unwrap_or_else(|_| panic!("error requesting: {text}"));

            insert_auth(
                self.pool.clone(),
                "ring",
                AuthState {
                    auth_token: auth_res.access_token,
                    refresh_token: auth_res.refresh_token,
                    hardware_id: Uuid::new_v4().to_string(),
                },
            )
            .await;
            "Login successful".to_string()
        } else {
            res.text().await.unwrap()
        }
    }

    pub async fn refresh_auth_token(&self) {
        let refresh_result = self.request_auth_token("", "", "").await;
        info!("refresh_result: {refresh_result}");
    }

    pub async fn request(
        &self,
        path: &str,
        method: Method,
    ) -> Result<Response, RingRestClientError> {
        let request_body = HashMap::from([("client_id", "ring_official_android")]);
        let auth_value = format!("{}{}", "Bearer ", &self.state.auth_token);

        let res = self
            .client
            .request(method, path)
            .json(&request_body)
            .header("authorization", auth_value)
            .header("hardware_id", &self.state.hardware_id)
            .header("User-Agent", "android:com.ringapp")
            .send()
            .await
            .map_err(|e| {
                RingRestClientError::InternalError(RingRestClientInternalError::Request(e))
            })?;

        let status = res.status();
        match status {
            _ if status.is_success() => Ok(res),
            StatusCode::UNAUTHORIZED => Err(RingRestClientError::Unauthorized),
            x => Err(RingRestClientError::InternalError(
                RingRestClientInternalError::UnexpectedResponseCode(x, res.text().await),
            )),
        }
    }

    pub async fn request_json<T>(
        &self,
        path: &str,
        method: Method,
    ) -> Result<T, RingRestClientError>
    where
        T: DeserializeOwned,
    {
        self.request(path, method)
            .await?
            .json::<T>()
            .await
            .map_err(|e| {
                RingRestClientError::InternalError(RingRestClientInternalError::Request(e))
            })
    }

    pub async fn get_locations(&self) -> Result<LocationsRes, RingRestClientError> {
        self.request_json::<LocationsRes>(&format!("{DEVICE_API_BASE_URL}locations"), Method::GET)
            .await
    }

    pub async fn get_devices(&self) -> Result<DevicesRes, RingRestClientError> {
        self.request_json::<DevicesRes>(&format!("{CLIENT_API_BASE_URL}ring_devices"), Method::GET)
            .await
    }

    pub async fn get_camera_events(
        &self,
        location_id: &str,
        device_id: &u64,
    ) -> Result<CameraEventsRes, RingRestClientError> {
        let camera_events_url =
            &format!("{CLIENT_API_BASE_URL}/locations/{location_id}/devices/{device_id}/events",);

        self.request_json::<CameraEventsRes>(camera_events_url, Method::GET)
            .await
    }

    pub async fn get_ws_url(&self) -> Result<String, RingRestClientError> {
        let socket_ticket = self
            .request_json::<SocketTicketRes>(
                &format!("{APP_API_BASE_URL}clap/ticket/request/signalsocket"),
                Method::POST,
            )
            .await?;

        let url = format!("wss://api.prod.signalling.ring.devices.a2z.com:443/ws?api_version=4.0&auth_type=ring_solutions&client_id=ring_site-3333&token={}", &socket_ticket.ticket);
        Ok(url)
    }

    pub async fn get_camera_snapshot(
        &self,
        id: &str,
    ) -> Result<(String, bytes::Bytes), RingRestClientError> {
        let snapshot_url = &format!("{SNAPSHOTS_API_BASE_URL}next/{id}");
        let res = self.request(snapshot_url, Method::GET).await?;

        let time_ms = res
            .headers()
            .get("x-time-millis")
            .ok_or(RingRestClientInternalError::HeadersMissingXTimeMillis)?
            .to_str()
            .map_err(RingRestClientInternalError::XTimeMillisHeaderNotString)?
            .parse::<f64>()
            .map_err(RingRestClientInternalError::XTimeMillisHeaderParseError)?;

        let snapshot_bytes = res
            .bytes()
            .await
            .map_err(RingRestClientInternalError::Request)?;

        let utc_time = DateTime::<Utc>::from_timestamp((time_ms / 1000.) as i64, 0)
            .ok_or(RingRestClientInternalError::HeadersMissingXTimeMillis)?;
        let est_time = utc_time.with_timezone(&Eastern);

        let formatted_time = est_time.format("%Y-%m-%d %I:%M:%S %p").to_string();

        Ok((formatted_time, snapshot_bytes))
    }

    pub async fn get_recordings(&self, id: &i64) -> Result<VideoSearchRes, RingRestClientError> {
        let date_from = get_start_of_today();
        let date_to = get_end_of_today();

        let recordings_url = format!(
            "{}video_search/history?doorbot_id={}&date_from={}&date_to={}&order=asc&api_version=11",
            CLIENT_API_BASE_URL, id, date_from, date_to
        );

        self.request_json::<VideoSearchRes>(&recordings_url, Method::GET)
            .await
    }

    pub async fn subscribe_to_motion_events(&self, device_id: &u64) {
        let recordings_url = &format!("{CLIENT_API_BASE_URL}devices/{device_id}/motions_subscribe");
        println!("{recordings_url}");
        let res = self
            .request_json::<serde_json::Value>(recordings_url, Method::POST)
            .await
            .unwrap();
        println!("subscribe motion events: {res}");
    }
}

pub async fn get_ring_camera(
    ring_rest_client: &Arc<RingRestClient>,
    device: &Doorbot,
) -> RingCamera {
    let snapshot_res = ring_rest_client
        .get_camera_snapshot(&device.id.to_string())
        .await;

    let snapshot_values = snapshot_res.unwrap_or(("".to_owned(), bytes::Bytes::new()));

    let image_base64 = base64.encode(snapshot_values.1);
    let videos = ring_rest_client
        .get_recordings(&device.id)
        .await
        .unwrap_or(VideoSearchRes {
            video_search: Vec::new(),
        });

    RingCamera {
        id: device.id,
        description: device.description.to_string(),
        snapshot: RingCameraSnapshot {
            image: image_base64,
            timestamp: snapshot_values.0,
        },
        health: device.health.battery_percentage,
        videos,
    }
}

/// Gets the UTC timestamp for the start of the local timezone's current day
fn get_start_of_today() -> i64 {
    Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("Date should be valid with non-invalid params")
        .and_utc()
        .timestamp_millis()
}

fn get_end_of_today() -> i64 {
    get_start_of_today() + Duration::days(1).num_milliseconds() - 1
}
