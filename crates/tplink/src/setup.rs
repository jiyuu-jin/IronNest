//! Putting a new TP-Link Kasa device on a Wi-Fi network without the Kasa app.
//!
//! A Kasa device in setup mode (fresh, or held reset) opens an **open** access
//! point named `TP-LINK_<model>_XXXX` (newer ones `Kasa_…`) and answers its
//! local protocol at [`SETUP_ADDRESS`]:[`PORT`]. Asked [`SYSINFO_REQUEST`] it
//! says what it is ([`parse_sysinfo`]); told a network's name and password
//! with [`join_request`], it answers `err_code: 0`, closes its access point
//! and joins that network. No account and no cloud.
//!
//! The bytes — the frame, the requests, the reading of the answers — are pure
//! functions with tests. Joining the access point is the caller's (it needs a
//! radio); [`exchange`] is the one blocking request/answer over TCP, usable
//! on the setup network and on the home network alike.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// Where a Kasa device in setup mode answers.
pub const SETUP_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 1);
/// The local protocol's TCP port.
pub const PORT: u16 = 9999;
/// An address a client can take on a device's setup network: the device is
/// .1 and hands out leases from its own pool, and nothing else is on it, so
/// a fixed address does as well as a DHCP client.
pub const CLIENT_ADDRESS: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 239);

/// The autokey cipher's first key.
const KEY: u8 = 0xAB;

/// Whether a network name is a Kasa device's setup access point.
pub fn is_setup_ssid(ssid: &str) -> bool {
    ssid.starts_with("TP-LINK_") || ssid.starts_with("Kasa_")
}

/// One request as the device reads it over TCP: a 4-byte big-endian length,
/// then the JSON under the autokey XOR (each byte's key is the previous
/// ciphertext byte).
pub fn encode_frame(json: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + json.len());
    out.extend_from_slice(&(json.len() as u32).to_be_bytes());
    let mut key = KEY;
    for byte in json.bytes() {
        key ^= byte;
        out.push(key);
    }
    out
}

/// The JSON in an answer, once `bytes` holds the whole frame; `None` while
/// more is still to come.
pub fn decode_frame(bytes: &[u8]) -> Option<Vec<u8>> {
    let length = u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    let body = bytes.get(4..4 + length)?;
    let mut key = KEY;
    Some(
        body.iter()
            .map(|&byte| {
                let plain = byte ^ key;
                key = byte;
                plain
            })
            .collect(),
    )
}

/// Why a network cannot be handed to a device, before anything is sent.
pub fn check_network(ssid: &str, password: &str) -> Result<(), String> {
    if ssid.is_empty() || ssid.len() > 32 {
        return Err("a Wi-Fi network name is 1 to 32 bytes".to_string());
    }
    if !password.is_empty() && !(8..=63).contains(&password.len()) {
        return Err("a WPA2 password is 8 to 63 characters".to_string());
    }
    Ok(())
}

/// The request telling a device to join `ssid`. `key_type` 3 is WPA2; an
/// empty password is an open network (0).
pub fn join_request(ssid: &str, password: &str) -> Result<String, String> {
    check_network(ssid, password)?;
    let key_type = if password.is_empty() { 0 } else { 3 };
    Ok(serde_json::json!({
        "netif": {"set_stainfo": {"ssid": ssid, "password": password, "key_type": key_type}}
    })
    .to_string())
}

/// Whether the device accepted the network. A refusal carries its own
/// message; anything unreadable is a failure, never assumed success.
pub fn check_answer(json: &[u8]) -> Result<(), String> {
    let answer: serde_json::Value =
        serde_json::from_slice(json).map_err(|_| "the device's answer was not JSON".to_string())?;
    let reply = answer
        .pointer("/netif/set_stainfo")
        .ok_or_else(|| "the device's answer did not mention the network".to_string())?;
    match reply.get("err_code").and_then(serde_json::Value::as_i64) {
        Some(0) => Ok(()),
        Some(code) => Err(
            match reply.get("err_msg").and_then(serde_json::Value::as_str) {
                Some(message) => format!("the device refused the network: {message} ({code})"),
                None => format!("the device refused the network (error {code})"),
            },
        ),
        None => Err("the device's answer did not say whether it took the network".to_string()),
    }
}

/// The request every Kasa device answers with what it is.
pub const SYSINFO_REQUEST: &str = r#"{"system":{"get_sysinfo":{}}}"#;

/// What a Kasa device says it is: enough to tell devices apart, in setup mode
/// or on the network, and to find one again when its address changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysInfo {
    /// Its MAC address, as [`normalize_mac`] writes one.
    pub mac: String,
    /// Its model, like `HS300(US)`.
    pub model: String,
    /// The name it carries (the setup network's name on a fresh device).
    pub alias: String,
    /// A power strip's outlets' names, in the order it lists them; empty for
    /// a single plug.
    pub outlets: Vec<String>,
}

impl SysInfo {
    /// The model without its region, like `HS300`.
    pub fn short_model(&self) -> &str {
        self.model.split('(').next().unwrap_or(&self.model).trim()
    }
}

/// A MAC address as six upper-case pairs joined by colons, from any of the
/// spellings a device or a person uses (`b0:be:76:12:34:56`,
/// `B0-BE-76-12-34-56`, `B0BE76123456`); `None` if it is not one.
pub fn normalize_mac(text: &str) -> Option<String> {
    let hex: String = text
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | '.' | ' '))
        .collect();
    if hex.len() != 12 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hex = hex.to_ascii_uppercase();
    Some(
        (0..6)
            .map(|pair| &hex[pair * 2..pair * 2 + 2])
            .collect::<Vec<_>>()
            .join(":"),
    )
}

/// Read a device's answer to [`SYSINFO_REQUEST`]. One that names no MAC is
/// refused: it could not be found again.
pub fn parse_sysinfo(json: &[u8]) -> Result<SysInfo, String> {
    let answer: serde_json::Value = serde_json::from_slice(json)
        .map_err(|_| "the device's system information was not JSON".to_string())?;
    let info = answer
        .pointer("/system/get_sysinfo")
        .ok_or_else(|| "the device's answer had no system information".to_string())?;
    let text = |key: &str| info.get(key).and_then(serde_json::Value::as_str);
    // Plugs say `mac`, some older devices `mic_mac` or `ethernet_mac`.
    let mac = ["mac", "mic_mac", "ethernet_mac"]
        .into_iter()
        .find_map(|key| text(key).and_then(normalize_mac))
        .ok_or_else(|| "the device did not say its MAC address".to_string())?;
    let model = text("model").unwrap_or_default().trim().to_string();
    let alias = text("alias").unwrap_or_default().trim().to_string();
    let outlets = info
        .get("children")
        .and_then(serde_json::Value::as_array)
        .map(|children| {
            children
                .iter()
                .map(|child| {
                    child
                        .get("alias")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(SysInfo {
        mac,
        model,
        alias,
        outlets,
    })
}

/// Send one request to a Kasa device at `address` and return the JSON it
/// answered, giving it `wait` to connect and as long again to answer.
pub fn exchange(address: SocketAddr, json: &str, wait: Duration) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect_timeout(&address, wait)
        .map_err(|error| format!("connecting to {address}: {error}"))?;
    let _ = stream.set_read_timeout(Some(wait));
    let _ = stream.set_write_timeout(Some(wait));
    stream
        .write_all(&encode_frame(json))
        .map_err(|error| format!("sending to {address}: {error}"))?;
    let mut answer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(json) = decode_frame(&answer) {
            return Ok(json);
        }
        // A device's answer is a few kilobytes; anything far larger is not one.
        if answer.len() > 1 << 20 {
            return Err(format!("{address} answered more than a device would"));
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Err(format!("{address} closed without answering")),
            Ok(read) => answer.extend_from_slice(&chunk[..read]),
            Err(error) => return Err(format!("reading {address}'s answer: {error}")),
        }
    }
}

/// Ask the device in setup mode one thing, once a client has joined its
/// network. Its address can take a moment to answer after association, so
/// the connection is tried a few times, a second apart.
pub fn ask_setup_device(json: &str) -> Result<Vec<u8>, String> {
    let address = std::net::SocketAddrV4::new(SETUP_ADDRESS, PORT).into();
    let mut last = String::new();
    for attempt in 0..5 {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(1));
        }
        match exchange(address, json, Duration::from_secs(4)) {
            Ok(answer) => return Ok(answer),
            Err(error) => last = error,
        }
    }
    Err(format!(
        "the device did not answer at {SETUP_ADDRESS} ({last})"
    ))
}

/// What the device on the setup network is.
pub fn read_setup_device() -> Result<SysInfo, String> {
    parse_sysinfo(&ask_setup_device(SYSINFO_REQUEST)?)
}

/// Tell the device on the setup network to join `ssid`.
pub fn hand_over(ssid: &str, password: &str) -> Result<(), String> {
    let request = join_request(ssid, password)?;
    check_answer(&ask_setup_device(&request)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_the_length_then_the_autokey_cipher() {
        // The well-known `get_sysinfo` frame every Kasa client sends.
        let frame = encode_frame(r#"{"system":{"get_sysinfo":{}}}"#);
        assert_eq!(
            frame,
            crate::protocol::encrypt_with_header(SYSINFO_REQUEST.as_bytes())
        );
        assert_eq!(&frame[..4], &[0, 0, 0, 29]);
        assert_eq!(&frame[4..8], &[0xd0, 0xf2, 0x81, 0xf8]);
        assert_eq!(
            decode_frame(&frame).expect("whole"),
            br#"{"system":{"get_sysinfo":{}}}"#
        );
    }

    #[test]
    fn a_partial_frame_is_waited_for() {
        let frame = encode_frame(r#"{"a":1}"#);
        assert!(decode_frame(&frame[..3]).is_none());
        assert!(decode_frame(&frame[..frame.len() - 1]).is_none());
        // Extra bytes after the frame are not part of it.
        let mut longer = frame.clone();
        longer.push(7);
        assert_eq!(decode_frame(&longer).expect("whole"), br#"{"a":1}"#);
    }

    #[test]
    fn the_join_request_names_the_network_and_its_security() {
        let request = join_request("Home \"Net\"", "correct horse").expect("valid");
        let value: serde_json::Value = serde_json::from_str(&request).expect("JSON");
        let info = &value["netif"]["set_stainfo"];
        assert_eq!(info["ssid"], "Home \"Net\"");
        assert_eq!(info["password"], "correct horse");
        assert_eq!(info["key_type"], 3);
        let open: serde_json::Value =
            serde_json::from_str(&join_request("Cafe", "").expect("open")).expect("JSON");
        assert_eq!(open["netif"]["set_stainfo"]["key_type"], 0);
        assert!(join_request("", "password1").is_err());
        assert!(join_request(&"x".repeat(33), "password1").is_err());
        assert!(join_request("Home", "short").is_err());
    }

    #[test]
    fn only_an_err_code_of_zero_is_accepted() {
        assert_eq!(
            check_answer(br#"{"netif":{"set_stainfo":{"mac":"x","err_code":0}}}"#),
            Ok(())
        );
        let refused = check_answer(
            br#"{"netif":{"set_stainfo":{"err_code":-2,"err_msg":"member not support"}}}"#,
        )
        .unwrap_err();
        assert!(refused.contains("member not support"), "{refused}");
        assert!(check_answer(br#"{"netif":{"set_stainfo":{}}}"#).is_err());
        assert!(check_answer(br#"{"system":{}}"#).is_err());
        assert!(check_answer(b"garbage").is_err());
    }

    #[test]
    fn a_strip_says_its_mac_model_and_outlets() {
        let info = parse_sysinfo(
            br#"{"system":{"get_sysinfo":{"model":"HS300(US)","alias":"TP-LINK_Power Strip_ABCD",
            "mac":"b0-be-76-12-34-56","child_num":2,
            "children":[{"id":"x00","alias":"Plug 1 "},{"id":"x01","alias":""}]}}}"#,
        )
        .expect("a strip");
        assert_eq!(info.mac, "B0:BE:76:12:34:56");
        assert_eq!(info.short_model(), "HS300");
        assert_eq!(info.alias, "TP-LINK_Power Strip_ABCD");
        assert_eq!(info.outlets, vec!["Plug 1".to_string(), String::new()]);
    }

    #[test]
    fn a_plug_has_no_outlets_and_an_older_device_its_own_mac_key() {
        let info = parse_sysinfo(
            br#"{"system":{"get_sysinfo":{"model":"HS100(US)","alias":"Lamp","mic_mac":"50C7BF0A0B0C"}}}"#,
        )
        .expect("a plug");
        assert_eq!(info.mac, "50:C7:BF:0A:0B:0C");
        assert!(info.outlets.is_empty());
        assert_eq!(info.short_model(), "HS100");
    }

    #[test]
    fn a_device_without_a_mac_is_refused() {
        assert!(parse_sysinfo(br#"{"system":{"get_sysinfo":{"model":"HS100"}}}"#).is_err());
        assert!(parse_sysinfo(br#"{"system":{"get_sysinfo":{"mac":"not a mac"}}}"#).is_err());
        assert!(parse_sysinfo(br#"{"system":{}}"#).is_err());
        assert!(parse_sysinfo(b"garbage").is_err());
    }

    #[test]
    fn macs_are_written_one_way() {
        assert_eq!(
            normalize_mac("b0:be:76:12:34:56").as_deref(),
            Some("B0:BE:76:12:34:56")
        );
        assert_eq!(
            normalize_mac(" B0BE76123456 ").as_deref(),
            Some("B0:BE:76:12:34:56")
        );
        assert_eq!(
            normalize_mac("b0be.7612.3456").as_deref(),
            Some("B0:BE:76:12:34:56")
        );
        assert_eq!(normalize_mac("B0:BE:76:12:34"), None);
        assert_eq!(normalize_mac("G0:BE:76:12:34:56"), None);
        assert_eq!(normalize_mac(""), None);
    }

    #[test]
    fn an_exchange_is_one_frame_each_way() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let device = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut asked = Vec::new();
            let mut chunk = [0u8; 256];
            while decode_frame(&asked).is_none() {
                let read = stream.read(&mut chunk).expect("read");
                asked.extend_from_slice(&chunk[..read]);
            }
            let reply = encode_frame(r#"{"system":{"get_sysinfo":{"mac":"B0BE76123456"}}}"#);
            // In two pieces, as a slow device sends it.
            stream.write_all(&reply[..3]).expect("write");
            std::thread::sleep(Duration::from_millis(20));
            stream.write_all(&reply[3..]).expect("write");
            decode_frame(&asked).expect("whole")
        });
        let answer = exchange(address, SYSINFO_REQUEST, Duration::from_secs(2)).expect("answered");
        assert_eq!(
            parse_sysinfo(&answer).expect("info").mac,
            "B0:BE:76:12:34:56"
        );
        assert_eq!(device.join().expect("device"), SYSINFO_REQUEST.as_bytes());
    }

    #[test]
    fn setup_networks_are_told_apart_from_everything_else() {
        assert!(is_setup_ssid("TP-LINK_Power Strip_ABCD"));
        assert!(is_setup_ssid("TP-LINK_Smart Plug_1A2B"));
        assert!(is_setup_ssid("Kasa_Plug_1A2B"));
        assert!(!is_setup_ssid("tp-link_lowercase"));
        assert!(!is_setup_ssid("Home WiFi"));
        assert!(!is_setup_ssid("TP-Link_Router"));
    }
}
