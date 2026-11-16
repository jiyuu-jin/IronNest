//! A client for Tuya (Smart Life) devices over their local protocol.
//!
//! A Tuya device takes local commands only when they are encrypted with its
//! **local key**, which Tuya's cloud hands out (the IoT platform's device
//! details, or tools that read it from an account). The caller hands the key
//! to [`Tuya::set_key`]; it is held in memory only and never echoed.
//!
//! The protocol framing is `rustuya`'s. One connection is kept per device,
//! with a heartbeat, and reused for every command while it is up; the device
//! pushes every change on it, so a command on a quiet session needs no status
//! query first. Many Tuya devices take one local connection at a time, so
//! while this client holds one, other local clients (the vendor app on the
//! same network) fall back to the cloud.
//!
//! The protocol version (3.3, 3.4 or 3.5) is worked out by trying, once per
//! device, and remembered. Discovery would announce it, but discovery is a
//! UDP broadcast that does not cross a router or leave a bridged container.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use rustuya_tokio::{Device, Event, Listener, Version};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

/// The port a Tuya device listens on.
pub const PORT: u16 = 6668;

/// Versions tried, newest first: a 3.4/3.5 device refuses the older framing
/// outright, while a 3.3 device simply never answers the newer handshake.
const VERSIONS: [Version; 3] = [Version::V3_5, Version::V3_4, Version::V3_3];

/// The light switch (`switch_led`) on Tuya lights, and the first switch on
/// plugs and strips.
const DP_LIGHT: u32 = 20;
const DP_FIRST: u32 = 1;

/// A local key as opened, or why it cannot be used. Nothing about the value
/// is ever echoed back.
pub fn check_key(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() != 16 {
        return Err(format!(
            "the local key is {} characters; a Tuya local key is 16",
            value.len()
        ));
    }
    Ok(value.to_string())
}

/// The data points a status reply carries: `dps` at the top for 3.3, under
/// `data` for 3.4 and 3.5.
pub fn dps_of(payload: &[u8]) -> Option<Map<String, Value>> {
    let value: Value = serde_json::from_slice(payload).ok()?;
    value
        .get("dps")
        .or_else(|| value.pointer("/data/dps"))
        .and_then(Value::as_object)
        .cloned()
}

/// Which data point switches the device: the one asked for, or the light
/// switch if it has one, or its first switch.
pub fn pick_dp(asked: Option<u32>, dps: &Map<String, Value>) -> Result<u32, String> {
    if let Some(dp) = asked {
        return match dps.get(&dp.to_string()) {
            Some(Value::Bool(_)) | None => Ok(dp),
            Some(other) => Err(format!(
                "its data point {dp} is {other}, not an on/off switch"
            )),
        };
    }
    for dp in [DP_LIGHT, DP_FIRST] {
        if matches!(dps.get(&dp.to_string()), Some(Value::Bool(_))) {
            return Ok(dp);
        }
    }
    let switches: Vec<&String> = dps
        .iter()
        .filter(|(_, value)| value.is_boolean())
        .map(|(dp, _)| dp)
        .collect();
    Err(if switches.is_empty() {
        "it reported no on/off switch".to_string()
    } else {
        format!(
            "it has no switch at 20 or 1 — put which one to use in Outlet (it has {})",
            switches
                .iter()
                .map(|dp| dp.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub struct Tuya {
    runtime: Runtime,
    /// Each device's local key as last opened, or why it could not be.
    keys: HashMap<String, Result<String, String>>,
    timeout: Duration,
    /// The version each device answered on.
    versions: HashMap<String, Version>,
    /// [`PORT`], except in tests.
    port: u16,
    /// The connection kept to each device: a command goes over a session
    /// already open (with its heartbeat), not a new TCP connection and key
    /// negotiation each time.
    open: HashMap<String, Session>,
}

/// One device's kept connection, and what it last said in full.
#[derive(Clone)]
struct Session {
    device: Device,
    address: IpAddr,
    /// Every data point as last known: a full query's answer, with what this
    /// client has set since merged in.
    dps: Map<String, Value>,
    /// The last frame the device had sent when `dps` was made whole. While
    /// it is still the last, nothing has changed on the device (it pushes
    /// every change on this session), so a command needs no query first.
    seen: Option<Vec<u8>>,
}

/// The last status frame a device sent on its session.
fn latest_frame(device: &Device) -> Option<Vec<u8>> {
    device
        .watch_status()
        .borrow()
        .as_ref()
        .map(|message| message.payload.clone())
}

/// A light's brightness data point and its range: `bright_value_v2` (22,
/// 10–1000) on current firmware, `bright_value` (3, 25–255) on the first.
pub fn brightness_dp(dps: &Map<String, Value>) -> Option<(u32, u32, u32)> {
    if dps.get("22").is_some_and(Value::is_number) {
        Some((22, 10, 1000))
    } else if dps.get("3").is_some_and(Value::is_number) {
        Some((3, 25, 255))
    } else {
        None
    }
}

/// A percentage (1–100) on a data point's range, 1% at its bottom.
pub fn scale_brightness(percent: u32, low: u32, high: u32) -> u32 {
    let percent = percent.clamp(1, 100);
    low + ((high - low) * (percent - 1) + 49) / 99
}

impl Tuya {
    pub fn new(timeout: Duration) -> Result<Self, String> {
        // Its own runtime with a worker thread: the protocol's connection runs
        // as a task, which must keep moving while this thread waits on it.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|error| format!("starting the Tuya runtime: {error}"))?;
        Ok(Self {
            runtime,
            keys: HashMap::new(),
            timeout,
            versions: HashMap::new(),
            port: PORT,
            open: HashMap::new(),
        })
    }

    /// Talk to devices on `port` instead of [`PORT`] (a simulator, a proxy).
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Hand over a device's key (or why it could not be had).
    pub fn set_key(&mut self, id: &str, key: Result<String, String>) {
        let key = key.and_then(|value| check_key(&value));
        if self.keys.get(id) != Some(&key) {
            // A new key may mean a re-paired device on another version, and
            // the open session was keyed with the old one.
            self.versions.remove(id);
            self.drop_connection(id);
        }
        self.keys.insert(id.to_string(), key);
    }

    /// Close and forget the connection kept to `id`.
    fn drop_connection(&mut self, id: &str) {
        if let Some(session) = self.open.remove(id) {
            self.runtime.block_on(session.device.close());
        }
    }

    /// Record what this client just set, so the next command needs no
    /// query for it.
    fn remember(&mut self, id: &str, dp: u32, value: Value) {
        if let Some(session) = self.open.get_mut(id) {
            session.dps.insert(dp.to_string(), value);
            session.seen = latest_frame(&session.device);
        }
    }

    fn key(&self, id: &str) -> Result<String, String> {
        self.keys
            .get(id)
            .cloned()
            .unwrap_or_else(|| Err(format!("no local key has been opened for {id}")))
    }

    /// Open a connection on one version and read the device's status.
    async fn open(
        id: &str,
        key: &str,
        (address, port): (IpAddr, u16),
        version: Version,
        timeout: Duration,
    ) -> Result<(Device, Listener, Map<String, Value>), String> {
        let device = Device::builder(id, key.as_bytes().to_vec())
            .address(address.to_string())
            .port(port)
            .version(version)
            .auto_reconnect(false)
            // Kept open between commands, so kept alive: a device drops a
            // silent session after about half a minute.
            .heartbeat(Some(Duration::from_secs(10)))
            .connect_timeout(timeout)
            .handshake_timeout(Some(timeout))
            .send_timeout(timeout)
            .connect()
            .map_err(|error| error.to_string())?;
        let opened = async {
            device
                .wait_connected(timeout)
                .await
                .map_err(|error| error.to_string())?;
            let mut events = device.listener();
            device.query().await.map_err(|error| error.to_string())?;
            let dps = next_dps(&mut events, timeout, |_| true).await?;
            Ok::<_, String>((events, dps))
        }
        .await;
        match opened {
            Ok((events, dps)) => Ok((device, events, dps)),
            Err(error) => {
                device.close().await;
                Err(error)
            }
        }
    }

    /// A connection to `id` at `address`, on the version it answered on last
    /// time, or on the first that works.
    fn connect(
        &mut self,
        id: &str,
        address: IpAddr,
    ) -> Result<(Device, Listener, Map<String, Value>), String> {
        let key = self.key(id)?;
        let timeout = self.timeout;
        // The kept connection, when it is still up and to the same address.
        // A fresh listener either way, so nothing heard before this command
        // counts as its answer.
        if let Some(session) = self.open.get(id).cloned() {
            if session.address == address && session.device.is_connected() {
                let device = session.device.clone();
                if latest_frame(&device) == session.seen {
                    // Nothing changed on the device since its state was whole.
                    return Ok((device.clone(), device.listener(), session.dps));
                }
                // It said something since (somebody used the app): ask.
                let asked = self.runtime.block_on(async {
                    let mut events = device.listener();
                    device.query().await.map_err(|error| error.to_string())?;
                    let dps = next_dps(&mut events, timeout, |_| true).await?;
                    Ok::<_, String>((events, dps))
                });
                if let Ok((events, dps)) = asked {
                    let mut whole = session.dps.clone();
                    whole.extend(dps);
                    let seen = latest_frame(&device);
                    self.open.insert(
                        id.to_string(),
                        Session {
                            device: device.clone(),
                            address,
                            dps: whole.clone(),
                            seen,
                        },
                    );
                    return Ok((device, events, whole));
                }
            }
            self.drop_connection(id);
        }
        let known = self.versions.get(id).copied();
        let tries: Vec<Version> = match known {
            Some(version) => vec![version],
            None => VERSIONS.to_vec(),
        };
        let mut last = String::new();
        for version in tries {
            match self.runtime.block_on(Self::open(
                id,
                &key,
                (address, self.port),
                version,
                timeout,
            )) {
                Ok(opened) => {
                    self.versions.insert(id.to_string(), version);
                    self.open.insert(
                        id.to_string(),
                        Session {
                            device: opened.0.clone(),
                            address,
                            dps: opened.2.clone(),
                            seen: latest_frame(&opened.0),
                        },
                    );
                    return Ok(opened);
                }
                Err(error) => last = error,
            }
        }
        // A version that worked before and now fails: try them all next time
        // (the device may have been updated).
        self.versions.remove(id);
        Err(if known.is_some() {
            last
        } else {
            format!(
                "no answer on any protocol version — check the address and the local key ({last})"
            )
        })
    }

    pub fn switch(
        &mut self,
        id: &str,
        dp: Option<u32>,
        address: IpAddr,
        on: bool,
    ) -> Result<(), String> {
        let (device, events, dps) = self.connect(id, address)?;
        let dp = pick_dp(dp, &dps)?;
        self.set(id, (device, events), &dps, dp, Value::Bool(on), "switch")
    }

    /// Set a light's brightness, in percent (1–100) of its range.
    pub fn brightness(&mut self, id: &str, address: IpAddr, percent: u32) -> Result<(), String> {
        let (device, events, dps) = self.connect(id, address)?;
        let (dp, low, high) =
            brightness_dp(&dps).ok_or("it has no brightness to set (data point 22 or 3)")?;
        let value = Value::from(scale_brightness(percent, low, high));
        self.set(id, (device, events), &dps, dp, value, "brightness")
    }

    /// Write one data point, unless it already says `value`, and wait for the
    /// device to push it back — the protocol's only confirmation.
    fn set(
        &mut self,
        id: &str,
        (device, mut events): (Device, Listener),
        dps: &Map<String, Value>,
        dp: u32,
        value: Value,
        what: &str,
    ) -> Result<(), String> {
        if dps.get(&dp.to_string()) == Some(&value) {
            return Ok(());
        }
        let timeout = self.timeout;
        let wanted = value.clone();
        let result: Result<(), String> = self.runtime.block_on(async move {
            device
                .set_value(dp, &wanted)
                .await
                .map_err(|error| error.to_string())?;
            let key = dp.to_string();
            next_dps(&mut events, timeout, |dps| dps.get(&key) == Some(&wanted))
                .await
                .map(|_| ())
                .map_err(|_| format!("it did not confirm the {what}"))
        });
        match &result {
            Ok(()) => self.remember(id, dp, value),
            // A session that failed a command is not trusted with the next.
            Err(_) => self.drop_connection(id),
        }
        result
    }

    pub fn state(&mut self, id: &str, dp: Option<u32>, address: IpAddr) -> Result<bool, String> {
        let (_, _events, dps) = self.connect(id, address)?;
        let dp = pick_dp(dp, &dps)?;
        dps.get(&dp.to_string())
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("it did not report data point {dp}"))
    }

    /// Which addresses on the network accept a connection on the Tuya port.
    pub fn listening(&self, hosts: Vec<IpAddr>, timeout: Duration) -> Vec<IpAddr> {
        self.runtime.block_on(async {
            let mut tasks = tokio::task::JoinSet::new();
            for ip in hosts {
                tasks.spawn(async move {
                    let addr = SocketAddr::new(ip, PORT);
                    let open = tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr))
                        .await
                        .is_ok_and(|connected| connected.is_ok());
                    open.then_some(ip)
                });
            }
            let mut open = Vec::new();
            while let Some(done) = tasks.join_next().await {
                if let Ok(Some(ip)) = done {
                    open.push(ip);
                }
            }
            open.sort();
            open
        })
    }
}

/// The next status frame whose data points satisfy `wanted`.
async fn next_dps(
    events: &mut Listener,
    timeout: Duration,
    wanted: impl Fn(&Map<String, Value>) -> bool,
) -> Result<Map<String, Value>, String> {
    let wait = async {
        loop {
            match events.recv().await {
                Some(Event::Frame(message)) => {
                    if let Some(dps) = dps_of(&message.payload)
                        && wanted(&dps)
                    {
                        return Ok(dps);
                    }
                }
                Some(Event::Lagged(_)) => continue,
                None => return Err("the device closed the connection".to_string()),
            }
        }
    };
    tokio::time::timeout(timeout, wait)
        .await
        .map_err(|_| format!("no answer within {} seconds", timeout.as_secs()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_key_must_be_sixteen_characters_and_is_never_echoed() {
        assert_eq!(
            check_key(" 0123456789abcdef\n"),
            Ok("0123456789abcdef".to_string())
        );
        let short = check_key("0123").unwrap_err();
        assert!(short.contains("16"), "{short}");
        assert!(!short.contains("0123"), "a key is never echoed: {short}");
    }

    #[test]
    fn status_is_read_from_either_place_the_versions_put_it() {
        let old = br#"{"devId":"x","dps":{"20":true,"22":500}}"#;
        let new = br#"{"protocol":4,"t":1,"data":{"dps":{"1":false}}}"#;
        assert_eq!(dps_of(old).expect("3.3")["20"], json!(true));
        assert_eq!(dps_of(new).expect("3.4")["1"], json!(false));
        assert!(dps_of(b"not json").is_none());
        assert!(dps_of(br#"{"t":1}"#).is_none());
    }

    #[test]
    fn the_light_switch_is_picked_before_the_first_switch() {
        let dps = |value: Value| value.as_object().cloned().expect("object");
        assert_eq!(pick_dp(None, &dps(json!({"1": true, "20": false}))), Ok(20));
        assert_eq!(pick_dp(None, &dps(json!({"1": true, "2": false}))), Ok(1));
        assert_eq!(
            pick_dp(Some(2), &dps(json!({"1": true, "2": false}))),
            Ok(2)
        );
        // A data point that is not a switch is refused rather than written.
        assert!(pick_dp(Some(22), &dps(json!({"20": true, "22": 500}))).is_err());
        let none = pick_dp(None, &dps(json!({"3": true, "22": 500}))).unwrap_err();
        assert!(none.contains('3'), "{none}");
        assert!(pick_dp(None, &dps(json!({"22": 500}))).is_err());
    }

    #[test]
    fn a_device_with_no_key_is_refused_before_anything_is_sent() {
        let mut tuya = Tuya::new(Duration::from_millis(300)).expect("a runtime");
        let here = "10.255.255.1".parse().expect("ip");
        let error = tuya.switch("abc", None, here, true).unwrap_err();
        assert!(error.contains("no local key"), "{error}");
        // A key that could not be had says why.
        tuya.set_key("abc", Err("the key could not be read".to_string()));
        let error = tuya.switch("abc", None, here, true).unwrap_err();
        assert!(error.contains("could not be read"), "{error}");
    }

    #[test]
    fn a_device_that_never_answers_is_given_up_on() {
        let mut tuya = Tuya::new(Duration::from_millis(300)).expect("a runtime");
        tuya.set_key("abc", Ok("0123456789abcdef".to_string()));
        let started = std::time::Instant::now();
        assert!(
            tuya.state("abc", None, "10.255.255.1".parse().expect("ip"))
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    /// A Tuya light speaking 3.3, as a thread on a loopback port: it answers
    /// a status query, and a switch with an acknowledgement and then a push of
    /// its new status, as the firmware does. Frames it cannot read — the 3.4
    /// and 3.5 handshakes tried first — it ignores, as a 3.3 device does.
    type Lit = (
        u16,
        std::sync::Arc<std::sync::Mutex<Map<String, Value>>>,
        std::sync::Arc<std::sync::atomic::AtomicUsize>,
    );

    /// How many status queries the fake light has answered, process-wide
    /// per port (tests run in parallel, each on its own light).
    fn queries() -> &'static std::sync::Mutex<HashMap<u16, usize>> {
        static QUERIES: std::sync::OnceLock<std::sync::Mutex<HashMap<u16, usize>>> =
            std::sync::OnceLock::new();
        QUERIES.get_or_init(Default::default)
    }

    fn fake_light(key: &'static [u8]) -> Lit {
        use rustuya_core::crypto::TuyaCipher;
        use rustuya_core::frame::{Integrity, pack_55aa, peek_header};
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().expect("bound").port();
        let state = std::sync::Arc::new(std::sync::Mutex::new(
            json!({"20": false, "22": 500})
                .as_object()
                .cloned()
                .expect("object"),
        ));
        let shared = state.clone();
        let connections = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = connections.clone();
        std::thread::spawn(move || {
            let cipher = TuyaCipher::new(key).expect("a key");
            let reply = |stream: &mut std::net::TcpStream, seqno: u32, cmd: u32, json: &Value| {
                let mut body = vec![0, 0, 0, 0];
                body.extend(
                    cipher
                        .ecb_encrypt(json.to_string().as_bytes())
                        .expect("encrypted"),
                );
                let _ = stream.write_all(&pack_55aa(seqno, cmd, &body, Integrity::Crc32));
            };
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let mut buffer = Vec::new();
                let mut chunk = [0u8; 1024];
                while let Ok(read) = stream.read(&mut chunk) {
                    if read == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                    while let Ok(Some(header)) = peek_header(&buffer) {
                        if buffer.len() < header.frame_len {
                            break;
                        }
                        let frame: Vec<u8> = buffer.drain(..header.frame_len).collect();
                        let Ok(message) = rustuya_core::message::decode_message(
                            Version::V3_3,
                            &frame,
                            key,
                            false,
                        ) else {
                            continue;
                        };
                        let Ok(request) = serde_json::from_slice::<Value>(&message.payload) else {
                            continue;
                        };
                        match message.cmd {
                            0x0a => {
                                *queries().lock().expect("queries").entry(port).or_default() += 1;
                                let dps = Value::Object(shared.lock().expect("state").clone());
                                reply(
                                    &mut stream,
                                    message.seqno,
                                    0x0a,
                                    &json!({"devId": "x", "dps": dps}),
                                );
                            }
                            0x07 => {
                                let Some(set) = request.get("dps").and_then(Value::as_object)
                                else {
                                    continue;
                                };
                                shared.lock().expect("state").extend(set.clone());
                                let _ = stream.write_all(&pack_55aa(
                                    message.seqno,
                                    0x07,
                                    &[0, 0, 0, 0],
                                    Integrity::Crc32,
                                ));
                                reply(&mut stream, 0, 0x08, &json!({"devId": "x", "dps": set}));
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
        (port, state, connections)
    }

    #[test]
    fn a_3_3_light_is_found_read_and_switched_and_its_version_remembered() {
        let (port, state, _) = fake_light(b"0123456789abcdef");
        let mut tuya = Tuya::new(Duration::from_millis(400)).expect("a runtime");
        tuya.set_key("light1", Ok("0123456789abcdef".to_string()));
        tuya = tuya.with_port(port);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        assert_eq!(tuya.state("light1", None, here), Ok(false));
        assert_eq!(tuya.versions.get("light1"), Some(&Version::V3_3));
        tuya.switch("light1", None, here, true)
            .expect("switched on");
        assert_eq!(state.lock().expect("state")["20"], json!(true));
        // The brightness is left as it was: only the switch is written.
        assert_eq!(state.lock().expect("state")["22"], json!(500));
        assert_eq!(tuya.state("light1", None, here), Ok(true));
        tuya.switch("light1", None, here, false)
            .expect("switched off");
        assert_eq!(state.lock().expect("state")["20"], json!(false));
        // Asking for a data point that is not a switch writes nothing.
        assert!(tuya.switch("light1", Some(22), here, true).is_err());
        assert_eq!(state.lock().expect("state")["22"], json!(500));
    }

    /// One connection per light, kept: a switch is a command on a session
    /// already open, not a TCP connect and a key negotiation first.
    #[test]
    fn a_light_is_switched_over_the_connection_it_already_has() {
        let (port, state, connections) = fake_light(b"0123456789abcdef");
        let mut tuya = Tuya::new(Duration::from_millis(400)).expect("a runtime");
        tuya.set_key("light1", Ok("0123456789abcdef".to_string()));
        tuya = tuya.with_port(port);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        assert_eq!(tuya.state("light1", None, here), Ok(false));
        // The fake light, like the real one, ignores the 3.4 and 3.5
        // handshakes, which time out without a connection kept.
        let opened = connections.load(std::sync::atomic::Ordering::SeqCst);
        for on in [true, false, true] {
            tuya.switch("light1", None, here, on).expect("switched");
            assert_eq!(state.lock().expect("state")["20"], json!(on));
        }
        assert_eq!(tuya.state("light1", None, here), Ok(true));
        assert_eq!(
            connections.load(std::sync::atomic::Ordering::SeqCst),
            opened,
            "no new connection after the first"
        );
        // A new key is a new session.
        tuya.set_key("light1", Ok("fedcba9876543210".to_string()));
        assert!(tuya.state("light1", None, here).is_err());
    }

    /// Over a kept session, a command whose device has said nothing since
    /// its state was last whole goes straight out: no status query first.
    #[test]
    fn a_kept_session_switches_without_asking_first() {
        let (port, state, _) = fake_light(b"0123456789abcdef");
        let mut tuya = Tuya::new(Duration::from_millis(400)).expect("a runtime");
        tuya.set_key("light1", Ok("0123456789abcdef".to_string()));
        tuya = tuya.with_port(port);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        assert_eq!(tuya.state("light1", None, here), Ok(false));
        let asked = queries().lock().expect("q")[&port];
        tuya.switch("light1", None, here, true).expect("on");
        tuya.switch("light1", None, here, false).expect("off");
        assert_eq!(tuya.state("light1", None, here), Ok(false));
        assert_eq!(
            queries().lock().expect("q")[&port],
            asked,
            "no query per command"
        );
        assert_eq!(state.lock().expect("state")["20"], json!(false));
    }

    #[test]
    fn brightness_is_set_on_the_lights_own_range() {
        assert_eq!(scale_brightness(1, 10, 1000), 10);
        assert_eq!(scale_brightness(100, 10, 1000), 1000);
        assert_eq!(scale_brightness(40, 10, 1000), 400);
        assert_eq!(scale_brightness(100, 25, 255), 255);
        assert_eq!(scale_brightness(0, 10, 1000), 10, "clamped");
        let (port, state, _) = fake_light(b"0123456789abcdef");
        let mut tuya = Tuya::new(Duration::from_millis(400)).expect("a runtime");
        tuya.set_key("light1", Ok("0123456789abcdef".to_string()));
        tuya = tuya.with_port(port);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        tuya.brightness("light1", here, 40).expect("dimmed");
        assert_eq!(state.lock().expect("state")["22"], json!(400));
        // The switch is left as it was.
        assert_eq!(state.lock().expect("state")["20"], json!(false));
        tuya.brightness("light1", here, 100).expect("full");
        assert_eq!(state.lock().expect("state")["22"], json!(1000));
    }

    #[test]
    fn the_wrong_key_reads_and_switches_nothing() {
        let (port, state, _) = fake_light(b"0123456789abcdef");
        let mut tuya = Tuya::new(Duration::from_millis(300)).expect("a runtime");
        tuya.set_key("light1", Ok("fedcba9876543210".to_string()));
        tuya = tuya.with_port(port);
        let here: IpAddr = "127.0.0.1".parse().expect("ip");
        assert!(tuya.switch("light1", None, here, true).is_err());
        assert_eq!(state.lock().expect("state")["20"], json!(false));
        assert!(tuya.versions.is_empty());
    }
}
