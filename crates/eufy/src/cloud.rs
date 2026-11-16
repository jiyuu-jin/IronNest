//! Signing in to the eufy cloud and listing the account's devices.
//!
//! The flow: `estimate_domain` finds the account's region shard; one key
//! exchange per region gives the session key every later call is encrypted
//! and signed with; `/passport/login` (password encrypted to the server's
//! fixed key) gives the auth token, after which the key is exchanged again
//! with the token so the server binds it to the user. Two-factor sign-in
//! sends a code (by email) and finishes with [`Cloud::submit_code`]. The
//! token and session key can be kept ([`Session`]) and resumed, so a
//! restart does not sign in (or ask for a code) again.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::crypto::{self, SessionKey};

/// The app identity the API expects.
const APP_NAME: &str = "eufy_mega";
const APP_VERSION: &str = "6.0.41_26142";

/// How to sign in.
#[derive(Clone)]
pub struct Config {
    pub email: String,
    pub password: String,
    /// The account's two-letter country code ("US", "GB", "DE").
    pub country: String,
    /// The phone model reported as this install's device.
    pub phone_model: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("country", &self.country)
            .field("phone_model", &self.phone_model)
            .finish_non_exhaustive()
    }
}

impl Config {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.trim().to_string(),
            password: password.to_string(),
            country: "US".to_string(),
            phone_model: "Pixel 8".to_string(),
        }
    }

    /// A stable per-install id the token is bound to.
    fn openudid(&self) -> String {
        crypto::md5_hex(&format!("eufy-mega:{}", self.email))[..16].to_string()
    }
}

/// A signed-in session, to keep and resume.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub region: String,
    pub user_id: String,
    pub account_user_id: String,
    pub auth_token: String,
    pub key: SessionKey,
}

/// Where a sign-in stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Login {
    /// Signed in.
    Done,
    /// A code was sent (by email); finish with [`Cloud::submit_code`].
    CodeSent,
    /// The cloud wants a picture puzzle answered first (a data: URL).
    Captcha { id: String, image: String },
}

/// A device on the account, as the cloud lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    pub serial: String,
    pub name: String,
    pub device_type: i64,
    pub model: String,
    /// The station the device belongs to (itself, for a stand-alone device).
    pub station_serial: String,
    /// The peer-to-peer id, when the device takes P2P sessions.
    pub p2p_did: Option<String>,
    /// Its parameters as last reported: id → value.
    pub params: BTreeMap<u32, String>,
    /// The listing as it came.
    pub raw: Value,
}

impl Device {
    /// A lock's state: `Some(true)` locked, `Some(false)` unlocked, `None`
    /// when it has not reported one (parameter 6000: "4" locked, "3"
    /// unlocked).
    pub fn locked(&self) -> Option<bool> {
        match self.params.get(&6000).map(|value| value.trim()) {
            Some("4") => Some(true),
            Some("3") => Some(false),
            _ => None,
        }
    }

    /// A battery's charge in percent, when reported (parameter 6001 on smart
    /// locks, 1101 elsewhere).
    pub fn battery(&self) -> Option<u8> {
        [6001, 1101]
            .iter()
            .find_map(|id| self.params.get(id)?.trim().parse::<u8>().ok())
    }
}

/// The parameters out of a listing: `params` as a list of
/// `{param_type, param_value}`, or an object of id → value.
fn params_of(raw: &Value) -> BTreeMap<u32, String> {
    let text = |value: &Value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    };
    let mut params = BTreeMap::new();
    match raw.get("params") {
        Some(Value::Array(list)) => {
            for entry in list {
                let id = entry
                    .get("param_type")
                    .and_then(|id| id.as_u64().or_else(|| id.as_str()?.parse().ok()));
                if let (Some(id), Some(value)) = (id, entry.get("param_value").and_then(text)) {
                    params.insert(id as u32, value);
                }
            }
        }
        Some(Value::Object(map)) => {
            for (id, value) in map {
                if let (Ok(id), Some(value)) = (id.parse::<u32>(), text(value)) {
                    params.insert(id, value);
                }
            }
        }
        _ => {}
    }
    params
}

/// One listed device, or `None` for an entry with no serial.
pub fn device_of(raw: &Value) -> Option<Device> {
    let text = |key: &str| {
        raw.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let serial = text("device_sn");
    if serial.is_empty() {
        return None;
    }
    let station = text("station_sn");
    let p2p_did = Some(text("p2p_did")).filter(|did| !did.is_empty());
    Some(Device {
        name: text("device_name"),
        device_type: raw.get("device_type").and_then(Value::as_i64).unwrap_or(-1),
        model: text("device_model"),
        station_serial: if station.is_empty() {
            serial.clone()
        } else {
            station
        },
        p2p_did,
        params: params_of(raw),
        serial,
        raw: raw.clone(),
    })
}

fn now_secs() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
        .to_string()
}

/// A client of the eufy cloud.
pub struct Cloud {
    config: Config,
    region: String,
    bootstrap_host: Option<String>,
    key: Option<SessionKey>,
    auth: Option<(String, String, String)>,
    agent: ureq::Agent,
}

impl Cloud {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            region: "us-pr".to_string(),
            bootstrap_host: None,
            key: None,
            auth: None,
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(20))
                .build(),
        }
    }

    /// Pick up a kept session: no sign-in, no code.
    pub fn resume(config: Config, session: Session) -> Self {
        let mut cloud = Self::new(config);
        cloud.region = session.region;
        cloud.key = Some(session.key);
        cloud.auth = Some((session.user_id, session.account_user_id, session.auth_token));
        cloud
    }

    /// The session to keep, once signed in.
    pub fn session(&self) -> Option<Session> {
        let (user_id, account_user_id, auth_token) = self.auth.clone()?;
        Some(Session {
            region: self.region.clone(),
            user_id,
            account_user_id,
            auth_token,
            key: self.key.clone()?,
        })
    }

    fn base_headers(&self) -> Vec<(&'static str, String)> {
        let country = self.config.country.to_uppercase();
        vec![
            ("app-name", APP_NAME.to_string()),
            ("app-version", APP_VERSION.to_string()),
            ("app_version", APP_VERSION.to_string()),
            ("os-type", "android".to_string()),
            ("os_type", "android".to_string()),
            ("os-version", "36".to_string()),
            ("os_version", "36".to_string()),
            ("phone-model", self.config.phone_model.clone()),
            ("phone_model", self.config.phone_model.clone()),
            ("model-type", "PHONE".to_string()),
            ("country", country.clone()),
            ("ab_code", country),
            ("openudid", self.config.openudid()),
            ("language", "en".to_string()),
            ("test-flag", "false".to_string()),
            ("user-agent", "ktor-client".to_string()),
            ("accept", "application/json".to_string()),
        ]
    }

    fn auth_headers(&self) -> Vec<(&'static str, String)> {
        match &self.auth {
            Some((user_id, account_user_id, token)) => {
                let gtoken_id = if account_user_id.is_empty() {
                    user_id
                } else {
                    account_user_id
                };
                vec![
                    ("x-auth-token", token.clone()),
                    ("authorization", token.clone()),
                    ("gtoken", crypto::gtoken(gtoken_id)),
                ]
            }
            None => Vec::new(),
        }
    }

    /// POST and read the JSON envelope, whatever the HTTP status.
    fn post(
        &self,
        url: &str,
        body: &str,
        headers: &[(&str, String)],
    ) -> Result<(u16, Value), String> {
        let mut request = self.agent.post(url);
        for (name, value) in headers {
            request = request.set(name, value);
        }
        let (status, text) = match request.send_string(body) {
            Ok(response) => (
                response.status(),
                response.into_string().unwrap_or_default(),
            ),
            Err(ureq::Error::Status(status, response)) => {
                (status, response.into_string().unwrap_or_default())
            }
            Err(error) => return Err(format!("reaching {url}: {error}")),
        };
        Ok((
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        ))
    }

    /// Find the account's region shard.
    fn estimate_domain(&mut self) -> Result<(), String> {
        let url = format!(
            "https://mega-{}.eufy.com/passport/estimate_domain",
            self.region
        );
        let mut headers = self.base_headers();
        headers.push(("content-type", "application/json".to_string()));
        let body = json!({"ab": self.config.country.to_lowercase(), "mode": 1}).to_string();
        let (_, envelope) = self.post(&url, &body, &headers)?;
        let data = envelope.get("data").cloned().unwrap_or(Value::Null);
        let domain = ["domain", "host"]
            .iter()
            .find_map(|key| data.get(key).and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        let blob = data.to_string();
        if blob.contains("-eu-") {
            self.region = "eu-pr".to_string();
        } else if blob.contains("-us-") {
            self.region = "us-pr".to_string();
        }
        if !domain.is_empty() && !domain.starts_with("mega-") {
            self.bootstrap_host = Some(domain);
        }
        Ok(())
    }

    /// Exchange a session key with the API, as the current user if any.
    fn exchange_key(&mut self) -> Result<(), String> {
        let host = self
            .bootstrap_host
            .clone()
            .unwrap_or_else(|| format!("app-openapi-{}.eufy.com", self.region));
        let exchange = crypto::prepare_key_exchange(crypto::MEGA_LOCAL_KEY_HEX, &now_secs())?;
        let mut headers = self.base_headers();
        headers.extend([
            ("x-encryption-info", "algo_ecdh".to_string()),
            ("x-replay-info", "replay".to_string()),
            ("x-key-ident", exchange.key_ident.clone()),
            ("x-request-ts", exchange.ts.clone()),
            ("x-request-once", exchange.once.clone()),
            ("x-signature", exchange.signature.clone()),
            ("content-type", "application/json".to_string()),
        ]);
        headers.extend(self.auth_headers());
        let body = json!({"client_public_key": exchange.encrypted_client_public_key}).to_string();
        let (status, envelope) = self.post(
            &format!("https://{host}/openapi/oauth/key/exchange"),
            &body,
            &headers,
        )?;
        let server = envelope
            .pointer("/data/server_public_key")
            .and_then(Value::as_str)
            .filter(|_| status == 200 && envelope.get("code").and_then(Value::as_i64) == Some(0))
            .ok_or_else(|| {
                format!(
                    "the key exchange was refused ({status}): {}",
                    envelope_message(&envelope)
                )
            })?;
        self.key = Some(exchange.finish(server)?);
        Ok(())
    }

    /// An encrypted, signed POST to `host`; the decrypted `data`.
    fn post_signed(
        &mut self,
        host: &str,
        path: &str,
        body: &Value,
        authed: bool,
    ) -> Result<Value, String> {
        if self.key.is_none() {
            self.exchange_key()?;
        }
        let key = self.key.clone().expect("exchanged");
        let encrypted = crypto::encrypt_body(body.to_string().as_bytes(), &key.share_key)?;
        let text_first = path.contains("/passport/");
        let mut last = String::new();
        for content_type in if text_first {
            ["text/plain", "application/json"]
        } else {
            ["application/json", "text/plain"]
        } {
            let ts = now_secs();
            let once = crypto::gen_id();
            let mut headers = self.base_headers();
            headers.extend([
                ("content-type", content_type.to_string()),
                ("x-encryption-info", "algo_ecdh".to_string()),
                ("x-key-ident", key.key_ident.clone()),
                ("x-request-ts", ts.clone()),
                ("x-request-once", once.clone()),
                (
                    "x-signature",
                    crypto::sign_request(&key.share_key, &ts, &once, Some(&encrypted)),
                ),
                ("x-replay-info", "replay".to_string()),
            ]);
            if authed {
                headers.extend(self.auth_headers());
            }
            let (status, envelope) =
                self.post(&format!("https://{host}{path}"), &encrypted, &headers)?;
            let code = envelope.get("code").and_then(Value::as_i64);
            if status == 200 && code == Some(0) {
                return match envelope.get("data") {
                    Some(Value::String(data)) if !data.is_empty() => {
                        let plain = crypto::decrypt_body(data, &key.share_key)?;
                        Ok(serde_json::from_slice(&plain)
                            .unwrap_or(Value::String(String::from_utf8_lossy(&plain).into())))
                    }
                    Some(data) => Ok(data.clone()),
                    None => Ok(Value::Null),
                };
            }
            last = format!(
                "{path} failed ({status}/{}): {}",
                code.unwrap_or(-1),
                envelope_message(&envelope)
            );
            // A wrong content type is answered "signature invalid" (4416) or 400; anything else is final.
            if !(status == 400 || code == Some(4416) || code == Some(10000)) {
                break;
            }
        }
        Err(last)
    }

    fn passport_host(&self) -> String {
        format!("app-passport-{}.eufy.com", self.region)
    }

    fn login_body(&self, extra: Value) -> Result<Value, String> {
        let (public_key, password) = crypto::encrypt_login_password(&self.config.password)?;
        let mut body = json!({
            "email": self.config.email,
            "password": password,
            "ab": self.config.country.to_uppercase(),
            "client_secret_info": {"public_key": public_key},
            "answer": "", "captcha_id": "", "verify_code": "", "login_id": "",
        });
        if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            body.extend(extra.clone());
        }
        Ok(body)
    }

    fn login_with(&mut self, body: Value, verifying: bool) -> Result<Login, String> {
        if self.bootstrap_host.is_none() && !verifying {
            self.estimate_domain()?;
        }
        let host = self.passport_host();
        let authed = self.auth.is_some();
        let answer = match self.post_signed(&host, "/passport/login", &body, authed) {
            Ok(answer) => answer,
            Err(error)
                if error.contains("/26006)")
                    || error.contains("/26052)")
                    || error.contains("captcha") =>
            {
                let captcha =
                    self.post_signed(&host, "/passport/generate/captcha", &json!({}), false)?;
                return Ok(Login::Captcha {
                    id: captcha
                        .get("captcha_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    image: captcha
                        .get("item")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
            Err(error) => return Err(error),
        };
        let text = |keys: &[&str]| {
            keys.iter()
                .find_map(|key| answer.get(*key).and_then(Value::as_str))
                .map(str::to_string)
        };
        let user_id =
            text(&["ap_cloud_user_id", "user_id", "userId"]).ok_or("the sign-in gave no user")?;
        let token = text(&["auth_token", "token"]).ok_or("the sign-in gave no token")?;
        let account = text(&["user_id", "userId"]).unwrap_or_else(|| user_id.clone());
        self.auth = Some((user_id, account, token));
        let pending = answer
            .pointer("/fa_info/info")
            .and_then(Value::as_str)
            .is_some_and(|info| !info.is_empty());
        if pending && !verifying {
            let push = format!("app-push-{}.eufy.com", self.region);
            self.post_signed(
                &push,
                "/app/sendmsg/verify_code",
                &json!({"message_type": 2, "biz_type": 1004}),
                true,
            )?;
            return Ok(Login::CodeSent);
        }
        // Bind the session key to the user.
        self.key = None;
        self.exchange_key()?;
        Ok(Login::Done)
    }

    /// Sign in with the email and password.
    pub fn login(&mut self) -> Result<Login, String> {
        let body = self.login_body(json!({}))?;
        self.login_with(body, false)
    }

    /// Finish a two-factor sign-in with the code that was sent.
    pub fn submit_code(&mut self, code: &str) -> Result<Login, String> {
        let body = self.login_body(json!({"verify_code": code.trim(), "login_id": ""}))?;
        self.login_with(body, true)
    }

    /// Answer a captcha, then sign in.
    pub fn solve_captcha(&mut self, id: &str, answer: &str) -> Result<Login, String> {
        let body = self.login_body(json!({"captcha_id": id, "answer": answer.trim()}))?;
        self.login_with(body, false)
    }

    /// An authed call to `app-{service}-{region}.eufy.com{path}`.
    pub fn request(&mut self, service: &str, path: &str, body: &Value) -> Result<Value, String> {
        if self.auth.is_none() {
            return Err("not signed in".to_string());
        }
        let host = format!("app-{service}-{}.eufy.com", self.region);
        self.post_signed(&host, path, body, true)
    }

    /// Every device on the account. The device list is scoped by house in a
    /// way no one call covers, so the unscoped list and every house's list
    /// are merged by serial.
    pub fn devices(&mut self) -> Result<Vec<Device>, String> {
        let mut bodies = vec![json!({})];
        if let Ok(houses) = self.request("house", "/app/house/get_house_list", &json!({})) {
            for house in houses
                .get("house_infos")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(id) = house.get("house_id") {
                    bodies.push(json!({"house_id": id}));
                }
            }
        }
        let mut seen: BTreeMap<String, Device> = BTreeMap::new();
        let mut first_error = None;
        for body in bodies {
            match self.request("house", "/app/house/get_devs_list", &body) {
                Ok(listing) => {
                    for raw in listing
                        .get("devices")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        if let Some(device) = device_of(raw) {
                            seen.insert(device.serial.clone(), device);
                        }
                    }
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        match (seen.is_empty(), first_error) {
            (true, Some(error)) => Err(error),
            _ => Ok(seen.into_values().collect()),
        }
    }
}

fn envelope_message(envelope: &Value) -> String {
    envelope
        .get("msg")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| envelope.to_string().chars().take(160).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listed_lock_reads_its_state_and_battery() {
        let raw = json!({
            "device_sn": "T8000P0000000000",
            "device_name": "Example lock",
            "device_type": 184,
            "station_sn": "",
            "p2p_did": "",
            "params": [
                {"param_type": 6000, "param_value": "4"},
                {"param_type": 6001, "param_value": "87"},
            ],
        });
        let device = device_of(&raw).expect("a device");
        assert_eq!(device.serial, "T8000P0000000000");
        assert_eq!(device.station_serial, "T8000P0000000000", "stand-alone");
        assert_eq!(device.p2p_did, None);
        assert_eq!(device.locked(), Some(true));
        assert_eq!(device.battery(), Some(87));
        let mut unlocked = raw.clone();
        unlocked["params"][0]["param_value"] = json!("3");
        assert_eq!(device_of(&unlocked).expect("device").locked(), Some(false));
        unlocked["params"][0]["param_value"] = json!("1");
        assert_eq!(
            device_of(&unlocked).expect("device").locked(),
            None,
            "unknown is unknown"
        );
    }

    #[test]
    fn params_come_as_a_list_or_an_object_and_an_entry_needs_a_serial() {
        let object = device_of(
            &json!({"device_sn": "T8000P0000000001", "params": {"6000": 3, "1101": "55"}}),
        )
        .expect("a device");
        assert_eq!(object.locked(), Some(false));
        assert_eq!(object.battery(), Some(55));
        assert!(device_of(&json!({"device_name": "no serial"})).is_none());
    }

    #[test]
    fn the_password_never_appears_in_debug_output() {
        let config = Config::new("someone@example.com", "hunter2-secret");
        let shown = format!("{config:?}");
        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(!shown.contains("someone@"), "{shown}");
    }
}
