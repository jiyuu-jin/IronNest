//! Ring's cloud API, blocking: sign-in with two-factor, cameras, events,
//! snapshots, recordings. See the README.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const OAUTH: &str = "https://oauth.ring.com/oauth/token";
const API: &str = "https://api.ring.com/clients_api";
const SNAPS: &str = "https://app-snaps.ring.com/snapshots";
const CLIENT_ID: &str = "ring_official_android";
const USER_AGENT: &str = "android:com.ringapp";

/// What a later run needs to act for the account with no password and no
/// code: the refresh token and the hardware id it was issued to.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub refresh_token: String,
    pub hardware_id: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("refresh_token", &"…")
            .field("hardware_id", &self.hardware_id)
            .finish()
    }
}

/// How a sign-in went.
#[derive(Debug)]
pub enum SignIn {
    Done(Session),
    /// Ring sent a code; call again with it. The text says where it went.
    NeedsCode(String),
}

/// A fresh hardware id: a random UUID, which Ring binds the session to.
pub fn new_hardware_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("the system's random source");
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent(USER_AGENT)
        .build()
}

struct Token {
    access: String,
    refresh: String,
    good_until: Instant,
}

fn token_request(
    agent: &ureq::Agent,
    hardware_id: &str,
    grant: Value,
    code: Option<&str>,
) -> Result<Result<Token, (u16, Value)>, String> {
    let mut body = json!({"client_id": CLIENT_ID, "scope": "client"});
    if let (Value::Object(body), Value::Object(grant)) = (&mut body, grant) {
        body.extend(grant);
    }
    let answer = agent
        .post(OAUTH)
        .set("2fa-support", "true")
        .set("2fa-code", code.unwrap_or(""))
        .set("hardware_id", hardware_id)
        .send_json(body);
    let (status, text) = match answer {
        Ok(response) => (response.status(), response.into_string().unwrap_or_default()),
        Err(ureq::Error::Status(status, response)) => {
            (status, response.into_string().unwrap_or_default())
        }
        Err(error) => return Err(format!("reaching Ring: {error}")),
    };
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    if status != 200 {
        return Ok(Err((status, value)));
    }
    Ok(parse_token(&value))
}

fn parse_token(value: &Value) -> Result<Token, (u16, Value)> {
    let access = value.get("access_token").and_then(Value::as_str);
    let refresh = value.get("refresh_token").and_then(Value::as_str);
    let expires = value.get("expires_in").and_then(Value::as_u64).unwrap_or(3600);
    match (access, refresh) {
        (Some(access), Some(refresh)) => Ok(Token {
            access: access.to_string(),
            refresh: refresh.to_string(),
            good_until: Instant::now() + Duration::from_secs(expires.saturating_sub(60)),
        }),
        _ => Err((200, value.clone())),
    }
}

/// Where Ring says it sent a code, from its 412 answer.
pub fn code_prompt(answer: &Value) -> String {
    let state = answer.get("tsv_state").and_then(Value::as_str).unwrap_or("");
    let phone = answer.get("phone").and_then(Value::as_str).unwrap_or("");
    match state {
        "totp" => "Enter the code from your authenticator app.".to_string(),
        "" => "Ring sent a verification code.".to_string(),
        via if phone.is_empty() => format!("Ring sent a code by {via}."),
        via => format!("Ring sent a code to {phone} by {via}."),
    }
}

fn refusal(status: u16, answer: &Value) -> String {
    let said = answer
        .get("error_description")
        .or_else(|| answer.get("error"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| answer.to_string());
    if said.contains("too many requests") {
        return "Ring is refusing codes for now (at most 10 in 10 minutes); wait and try again"
            .to_string();
    }
    if said.starts_with("Verification Code") {
        return "that code is wrong or has expired".to_string();
    }
    format!("Ring refused ({status}): {said}")
}

/// Sign in. The first call (no code) normally answers `NeedsCode`; the
/// second, with the code, a `Session`. Use the same `hardware_id` for both.
pub fn sign_in(
    email: &str,
    password: &str,
    code: Option<&str>,
    hardware_id: &str,
) -> Result<SignIn, String> {
    let grant = json!({"grant_type": "password", "username": email, "password": password});
    match token_request(&agent(), hardware_id, grant, code)? {
        Ok(token) => Ok(SignIn::Done(Session {
            refresh_token: token.refresh,
            hardware_id: hardware_id.to_string(),
        })),
        Err((412, answer)) => Ok(SignIn::NeedsCode(code_prompt(&answer))),
        Err((status, answer)) => Err(refusal(status, &answer)),
    }
}

/// A camera (doorbell, stick-up or floodlight cam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    pub id: u64,
    pub name: String,
    /// Ring's model name, like `doorbell_v4` or `stickup_cam_v4`.
    pub kind: String,
    pub location_id: String,
    pub doorbell: bool,
}

/// One thing a camera saw or heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Ring's id for it, as text (it overflows a double).
    pub id: String,
    pub camera_id: u64,
    /// When, as Ring wrote it (ISO 8601, UTC).
    pub created_at: String,
    /// `motion`, `ding`, `on_demand`, …
    pub kind: String,
    pub person: bool,
    /// What the camera's vision said it was, when it said (`human`,
    /// `vehicle`, `package_delivery`, …).
    pub detection: Option<String>,
    /// A doorbell press nobody answered.
    pub missed: bool,
    /// Whether video was kept (needs a plan with recording).
    pub recorded: bool,
}

/// A kept recording, with the picture Ring made of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    pub id: String,
    pub created_at: String,
    pub kind: String,
    pub thumbnail_url: Option<String>,
    pub detection: Option<String>,
}

/// The cameras in a `ring_devices` answer.
pub fn parse_cameras(answer: &Value) -> Vec<Camera> {
    let mut cameras = Vec::new();
    for (list, doorbell) in [
        ("doorbots", true),
        ("authorized_doorbots", true),
        ("stickup_cams", false),
        ("other", false),
    ] {
        for device in answer.get(list).and_then(Value::as_array).into_iter().flatten() {
            let kind = device.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
            if list == "other" && !(kind.contains("camera") || kind.starts_with("doorbell")) {
                continue;
            }
            let Some(id) = device.get("id").and_then(Value::as_u64) else {
                continue;
            };
            if cameras.iter().any(|camera: &Camera| camera.id == id) {
                continue;
            }
            cameras.push(Camera {
                id,
                name: device
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                doorbell: doorbell || kind.starts_with("doorbell"),
                kind,
                location_id: match device.get("location_id") {
                    Some(Value::String(text)) => text.clone(),
                    Some(Value::Number(number)) => number.to_string(),
                    _ => String::new(),
                },
            });
        }
    }
    cameras
}

fn text_id(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn detection(event: &Value) -> Option<String> {
    event
        .pointer("/cv_properties/detection_type")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && *text != "null")
        .map(str::to_string)
}

/// The events in an `events` answer, and the key for the page after it.
pub fn parse_events(answer: &Value, camera_id: u64) -> (Vec<Event>, Option<String>) {
    let events = answer
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| {
            let id = text_id(event.get("ding_id_str")).or_else(|| text_id(event.get("ding_id")))?;
            let kind = event.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
            Some(Event {
                id,
                camera_id,
                created_at: event
                    .get("created_at")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                person: event
                    .pointer("/cv_properties/person_detected")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                detection: detection(event),
                missed: kind == "ding"
                    && event.get("state").and_then(Value::as_str) == Some("timed_out"),
                recorded: event.get("recorded").and_then(Value::as_bool).unwrap_or(false),
                kind,
            })
        })
        .collect();
    let next = answer
        .pointer("/meta/pagination_key")
        .and_then(Value::as_str)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    (events, next)
}

/// The recordings in a `video_search/history` answer.
pub fn parse_recordings(answer: &Value) -> Vec<Recording> {
    answer
        .get("video_search")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(Recording {
                id: text_id(item.get("ding_id"))?,
                created_at: match item.get("created_at") {
                    Some(Value::String(text)) => text.clone(),
                    Some(Value::Number(ms)) => ms.to_string(),
                    _ => String::new(),
                },
                kind: item.get("kind").and_then(Value::as_str).unwrap_or("").to_string(),
                thumbnail_url: item
                    .get("thumbnail_url")
                    .and_then(Value::as_str)
                    .filter(|url| !url.is_empty())
                    .map(str::to_string),
                detection: detection(item),
            })
        })
        .collect()
}

/// A signed-in account.
pub struct Ring {
    agent: ureq::Agent,
    hardware_id: String,
    refresh: String,
    token: Option<Token>,
    session_made: bool,
}

impl Ring {
    pub fn resume(session: Session) -> Self {
        Self {
            agent: agent(),
            hardware_id: session.hardware_id,
            refresh: session.refresh_token,
            token: None,
            session_made: false,
        }
    }

    /// The session as it stands now — the refresh token rotates, so keep
    /// this after using the account rather than the one it was resumed from.
    pub fn session(&self) -> Session {
        Session {
            refresh_token: self.refresh.clone(),
            hardware_id: self.hardware_id.clone(),
        }
    }

    fn access(&mut self) -> Result<String, String> {
        if let Some(token) = &self.token
            && Instant::now() < token.good_until
        {
            return Ok(token.access.clone());
        }
        let grant = json!({"grant_type": "refresh_token", "refresh_token": self.refresh});
        let token = match token_request(&self.agent, &self.hardware_id, grant, None)? {
            Ok(token) => token,
            Err((status, answer)) => {
                return Err(format!(
                    "the Ring session no longer works — sign in again ({})",
                    refusal(status, &answer)
                ));
            }
        };
        self.refresh = token.refresh.clone();
        let access = token.access.clone();
        self.token = Some(token);
        if !self.session_made {
            let body = json!({"device": {
                "hardware_id": self.hardware_id,
                "metadata": {"api_version": 11, "device_model": "ironnest"},
                "os": "android",
            }});
            self.agent
                .post(&format!("{API}/session"))
                .set("authorization", &format!("Bearer {access}"))
                .set("hardware_id", &self.hardware_id)
                .send_json(body)
                .map_err(|error| format!("starting a Ring session: {error}"))?;
            self.session_made = true;
        }
        Ok(access)
    }

    fn get(&mut self, url: &str, accept: &str) -> Result<ureq::Response, String> {
        let access = self.access()?;
        let answer = self
            .agent
            .get(url)
            .set("authorization", &format!("Bearer {access}"))
            .set("hardware_id", &self.hardware_id)
            .set("accept", accept)
            .call();
        match answer {
            Ok(response) => Ok(response),
            Err(ureq::Error::Status(401, _)) => {
                // The access token went stale early: once more, fresh.
                self.token = None;
                let access = self.access()?;
                self.agent
                    .get(url)
                    .set("authorization", &format!("Bearer {access}"))
                    .set("hardware_id", &self.hardware_id)
                    .set("accept", accept)
                    .call()
                    .map_err(|error| format!("asking Ring: {error}"))
            }
            Err(error) => Err(format!("asking Ring: {error}")),
        }
    }

    fn get_json(&mut self, url: &str) -> Result<Value, String> {
        self.get(url, "application/json")?
            .into_json()
            .map_err(|error| format!("reading Ring's answer: {error}"))
    }

    fn get_bytes(&mut self, url: &str) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(
            &mut self.get(url, "image/jpeg")?.into_reader().take_limit(),
            &mut bytes,
        )
        .map_err(|error| format!("reading a picture from Ring: {error}"))?;
        Ok(bytes)
    }

    pub fn cameras(&mut self) -> Result<Vec<Camera>, String> {
        Ok(parse_cameras(&self.get_json(&format!("{API}/ring_devices"))?))
    }

    /// A camera's latest events, newest first.
    pub fn events(&mut self, camera: &Camera, limit: u32) -> Result<Vec<Event>, String> {
        let url = format!(
            "{API}/locations/{}/devices/{}/events?limit={limit}",
            camera.location_id, camera.id
        );
        Ok(parse_events(&self.get_json(&url)?, camera.id).0)
    }

    /// A camera's kept recordings between two instants (ms since the epoch).
    pub fn recordings(
        &mut self,
        camera_id: u64,
        from_ms: i64,
        to_ms: i64,
    ) -> Result<Vec<Recording>, String> {
        let url = format!(
            "{API}/video_search/history?doorbot_id={camera_id}&date_from={from_ms}&date_to={to_ms}\
             &order=desc&api_version=11&includes%5B%5D=pva"
        );
        Ok(parse_recordings(&self.get_json(&url)?))
    }

    /// The picture at a thumbnail URL Ring handed out (already signed).
    pub fn thumbnail(&mut self, url: &str) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|error| format!("fetching a thumbnail: {error}"))?;
        std::io::Read::read_to_end(&mut response.into_reader().take_limit(), &mut bytes)
            .map_err(|error| format!("reading a thumbnail: {error}"))?;
        Ok(bytes)
    }

    /// The camera's latest still (JPEG).
    pub fn snapshot(&mut self, camera_id: u64) -> Result<Vec<u8>, String> {
        self.get_bytes(&format!("{SNAPS}/next/{camera_id}?extras=force"))
    }

    /// A link to an event's video, good for a while.
    pub fn recording_url(&mut self, event_id: &str) -> Result<String, String> {
        let answer =
            self.get_json(&format!("{API}/dings/{event_id}/share/play?disable_redirect=true"))?;
        answer
            .get("url")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| "Ring gave no link for that recording".to_string())
    }
}

/// The most a picture may be: a still is tens of kilobytes; anything near
/// this is not one.
const PICTURE_LIMIT: u64 = 8 * 1024 * 1024;

trait TakeLimit: std::io::Read + Sized {
    fn take_limit(self) -> std::io::Take<Self> {
        std::io::Read::take(self, PICTURE_LIMIT)
    }
}
impl<R: std::io::Read> TakeLimit for R {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cameras_come_from_every_list_once() {
        let answer = json!({
            "doorbots": [{"id": 1, "description": "Front", "kind": "doorbell_v4", "location_id": "loc"}],
            "authorized_doorbots": [{"id": 1, "description": "Front", "kind": "doorbell_v4", "location_id": "loc"}],
            "stickup_cams": [{"id": 2, "description": "Yard", "kind": "stickup_cam_v4", "location_id": "loc"}],
            "chimes": [{"id": 3, "description": "Chime", "kind": "chime"}],
            "other": [
                {"id": 4, "description": "Intercom", "kind": "intercom_handset_audio"},
                {"id": 5, "description": "Porch", "kind": "onvif_camera", "location_id": "loc"}
            ]
        });
        let cameras = parse_cameras(&answer);
        let ids: Vec<u64> = cameras.iter().map(|camera| camera.id).collect();
        assert_eq!(ids, [1, 2, 5]);
        assert!(cameras[0].doorbell);
        assert!(!cameras[1].doorbell);
        assert_eq!(cameras[1].name, "Yard");
    }

    #[test]
    fn events_keep_the_id_as_text_and_say_who_was_seen() {
        let answer = json!({
            "events": [
                {"ding_id": 7.3e18, "ding_id_str": "7300000000000000001", "created_at": "2026-01-02T03:04:05Z",
                 "kind": "motion", "recorded": true, "state": "completed",
                 "cv_properties": {"person_detected": true, "detection_type": "human"}},
                {"ding_id_str": "7300000000000000002", "created_at": "2026-01-02T03:00:00Z",
                 "kind": "ding", "state": "timed_out", "cv_properties": {"detection_type": null}},
                {"kind": "motion"}
            ],
            "meta": {"pagination_key": "next-page"}
        });
        let (events, next) = parse_events(&answer, 9);
        assert_eq!(events.len(), 2, "an event with no id is skipped");
        assert_eq!(events[0].id, "7300000000000000001");
        assert!(events[0].person);
        assert_eq!(events[0].detection.as_deref(), Some("human"));
        assert!(!events[0].missed);
        assert!(events[1].missed, "an unanswered ring");
        assert_eq!(events[1].detection, None);
        assert_eq!(events[1].camera_id, 9);
        assert_eq!(next.as_deref(), Some("next-page"));
    }

    #[test]
    fn recordings_carry_their_thumbnail() {
        let answer = json!({"video_search": [
            {"ding_id": "11", "created_at": 1767323045000_i64, "kind": "motion",
             "thumbnail_url": "https://example.invalid/t.jpg", "cv_properties": {"detection_type": "vehicle"}},
            {"ding_id": "12", "created_at": "2026-01-02T03:00:00Z", "kind": "ding", "thumbnail_url": ""}
        ]});
        let recordings = parse_recordings(&answer);
        assert_eq!(recordings[0].thumbnail_url.as_deref(), Some("https://example.invalid/t.jpg"));
        assert_eq!(recordings[0].created_at, "1767323045000");
        assert_eq!(recordings[0].detection.as_deref(), Some("vehicle"));
        assert_eq!(recordings[1].thumbnail_url, None);
    }

    #[test]
    fn the_code_prompt_says_where_it_went() {
        assert_eq!(
            code_prompt(&json!({"tsv_state": "sms", "phone": "+1xxxxxxx12"})),
            "Ring sent a code to +1xxxxxxx12 by sms."
        );
        assert!(code_prompt(&json!({"tsv_state": "totp"})).contains("authenticator"));
    }

    #[test]
    fn a_wrong_code_and_rate_limit_read_plainly() {
        assert_eq!(
            refusal(400, &json!({"error": "Verification Code is invalid or expired"})),
            "that code is wrong or has expired"
        );
        assert!(
            refusal(400, &json!({"error_description": "too many requests from dependency service"}))
                .contains("10 in 10 minutes")
        );
    }

    #[test]
    fn a_hardware_id_is_a_v4_uuid() {
        let id = new_hardware_id();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert_ne!(id, new_hardware_id());
    }

    #[test]
    fn a_session_never_prints_its_token() {
        let session = Session {
            refresh_token: "secret".into(),
            hardware_id: "hid".into(),
        };
        assert!(!format!("{session:?}").contains("secret"));
    }
}
