//! Locking and unlocking a smart lock.
//!
//! Which wire a lock takes is decided by its family, as mega-yfue/eufy-sdk
//! (Apache-2.0) decides it in `src/model/capabilities/lock.ts` and
//! `src/model/device-family.ts`:
//!
//! - the Video Smart Lock S330 (device type 55) takes its own `set-payload`
//!   command — **not ported**;
//! - the classic Wi-Fi lock (type 51 with a `T8520` serial whose seventh
//!   character is `2`) takes a per-command ECDH-wrapped "keyed payload" —
//!   **not ported**;
//! - every other lock takes the shared `ff09` frame ([`crate::ff09`]). This
//!   is the path ported here, and the one the Smart Lock C220 / Touch &
//!   Wi-Fi (`T8506`, device type 184) falls to.
//!
//! A device with a P2P id is driven over P2P (the SDK's `routeCommand`):
//! the session goes to the device's station — the device itself when it
//! stands alone — the level-2 key is negotiated, and the frame goes inside
//! a `SET_PAYLOAD` (1350) envelope
//! `{account_id, cmd: 1940, mChannel, mValue3: 0, payload: {apiCommand: 6018,
//! lock_payload, seq_num, time}}`, sent five times 200 ms apart as the SDK
//! sends every unacknowledged P2P control (`sendFf09Actuate` →
//! `sendSetPayloadEnvelope` → `replayLevel2Send`).
//!
//! **Verification.** The SDK verifies the `ff09` frame byte-exact against
//! captures from the T8531 video lock and the T85D0 garage door, and drives
//! lock/unlock over P2P on the T8531. It names no capture from a T8506: that
//! the T8506 takes `ff09` is the SDK's family fall-through, not something
//! it has shown on that hardware. The SDK also treats the P2P actuation as
//! fire-and-forget and reads no reply to it; what this port reads as the
//! acknowledgement (the transport ACK of the sends, a bare int32 result,
//! or a 1940 reply frame) is described on [`Actuation`], and the meaning of
//! a reply's status byte is not established.

use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddrV4, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::cloud::{Cloud, Device};
use crate::ff09;
use crate::p2p::{self, Event, Session, codec};

/// Sends of one actuation (the SDK's `DIRECT_CMD_SENDS`).
const SENDS: usize = 5;
const SEND_SPACING: Duration = Duration::from_millis(200);
/// How long after the last send replies are collected.
const REPLY_WINDOW: Duration = Duration::from_secs(3);

/// The eufy device types of locks and safes (the SDK's `LOCK_TYPES`).
const LOCK_TYPES: &[i64] = &[
    50, 51, 52, 53, 54, 55, 56, 57, 58, 140, 141, 142, 143, 180, 184, 189, 201, 202, 209,
];
/// The Video Smart Lock S330.
const VIDEO_SMART_LOCK: i64 = 55;
/// The Wi-Fi smart lock type, shared by the classic unit and later ones.
const LOCK_WIFI: i64 = 51;

/// The wire a lock's actuation takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    /// The shared `ff09` frame (ported).
    Ff09,
    /// The video lock's own `set-payload` (not ported).
    VideoLockPayload,
    /// The classic Wi-Fi lock's keyed payload (not ported).
    KeyedPayload,
}

/// The wire `device` takes, or why it cannot be locked here.
pub fn lock_wire(device: &Device) -> Result<Wire, String> {
    if !LOCK_TYPES.contains(&device.device_type) {
        return Err(format!("device type {} is not a lock", device.device_type));
    }
    if device.device_type == VIDEO_SMART_LOCK {
        return Ok(Wire::VideoLockPayload);
    }
    let serial = device.serial.as_bytes();
    if device.device_type == LOCK_WIFI
        && device.serial.starts_with("T8520")
        && serial.get(6) == Some(&b'2')
    {
        return Ok(Wire::KeyedPayload);
    }
    Ok(Wire::Ff09)
}

fn text<'a>(raw: &'a Value, pointer: &str) -> Option<&'a str> {
    raw.pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

/// The station a device's traffic goes through: `parent_sn` when it names
/// another device, else `station_sn`, else the device itself.
pub fn station_of(device: &Device) -> String {
    text(&device.raw, "/parent_sn")
        .filter(|parent| *parent != device.serial)
        .or_else(|| text(&device.raw, "/station_sn"))
        .unwrap_or(&device.serial)
        .to_string()
}

/// The lock's channel on its station (`device_channel`, 0 when a
/// stand-alone device states none). An attached device must state one.
pub fn channel_of(device: &Device, standalone: bool) -> Result<u8, String> {
    match device.raw.get("device_channel").and_then(Value::as_u64) {
        Some(channel) => {
            u8::try_from(channel).map_err(|_| format!("device_channel {channel} out of range"))
        }
        None if standalone => Ok(0),
        None => Err(format!(
            "{} states no channel on its station",
            device.serial
        )),
    }
}

/// Who the frame says acts: the lock's `member.admin_user_id` and
/// `member.short_user_id`, and the account's acting name.
pub fn identity_of(device: &Device, acting_name: &str) -> Result<ff09::Identity, String> {
    let admin = text(&device.raw, "/member/admin_user_id")
        .ok_or("the lock's record names no member.admin_user_id")?;
    let short = text(&device.raw, "/member/short_user_id")
        .ok_or("the lock's record names no member.short_user_id")?;
    if acting_name.is_empty() {
        return Err("no acting name: set Config::account_name or sign in with an email".into());
    }
    Ok(ff09::Identity {
        admin_user_id: admin.to_string(),
        user_name: acting_name.to_string(),
        short_user_id: short.to_string(),
        device_serial: device.serial.clone(),
    })
}

fn private_ipv4(text: &str) -> Option<Ipv4Addr> {
    let ip: Ipv4Addr = text.trim().parse().ok()?;
    ip.is_private().then_some(ip)
}

/// The freshest private IPv4 among a record's parameter values, else its
/// `local_ip` or `ip_addr` (the SDK's `freshestLanIp`).
pub fn lan_address(raw: &Value) -> Option<Ipv4Addr> {
    let from_params = raw
        .get("params")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|param| {
            let ip = private_ipv4(param.get("param_value")?.as_str()?)?;
            Some((
                param
                    .get("update_time")
                    .and_then(Value::as_i64)
                    .unwrap_or(0),
                ip,
            ))
        })
        .max_by_key(|(time, _)| *time)
        .map(|(_, ip)| ip);
    from_params.or_else(|| {
        ["local_ip", "ip_addr"]
            .iter()
            .find_map(|key| private_ipv4(raw.get(key)?.as_str()?))
    })
}

/// The lookup servers a record's `p2p_conn` (else `app_conn`) names,
/// resolved to IPv4.
pub fn lookup_servers(raw: &Value) -> Vec<SocketAddrV4> {
    let Some(conn) = text(raw, "/p2p_conn").or_else(|| text(raw, "/app_conn")) else {
        return Vec::new();
    };
    codec::decode_cloud_hosts(conn)
        .iter()
        .filter_map(|host| {
            (host.as_str(), codec::CLOUD_LOOKUP_PORT)
                .to_socket_addrs()
                .ok()?
                .find_map(|addr| match addr {
                    std::net::SocketAddr::V4(v4) => Some(v4),
                    _ => None,
                })
        })
        .collect()
}

/// The `SET_PAYLOAD` command fields for one frame: `cmd` 1940, `mValue3` 0,
/// and the `TRANSFER_PAYLOAD` payload.
pub fn transfer_command(frame: &ff09::Frame) -> (u32, Value, u32) {
    (ff09::CMD_TRANSFER_PAYLOAD, frame.transfer_payload(), 0)
}

/// What came back from an actuation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Actuation {
    /// How many of the sends the device acknowledged at the transport
    /// level (PPCS ACK of the DATA packet) — delivery, not execution.
    pub acknowledged_sends: usize,
    /// A bare int32 result the device answered on the lock's channel, if
    /// any (the SDK reads negative values as refusals: -104, -108, …).
    pub result_code: Option<i32>,
    /// A 1940 reply frame matched to this command's key time, decrypted,
    /// if the device sent one. Its status byte's meaning is unverified.
    pub reply: Option<ff09::Reply>,
}

/// Folds the session's events into what an actuation learned.
#[derive(Debug, Default)]
pub struct Replies {
    sent: BTreeSet<u16>,
    acked: BTreeSet<u16>,
    result: Option<i32>,
    reply_payload: Option<String>,
}

impl Replies {
    /// Note that a send went out under DATA sequence number `seq`.
    pub fn sent(&mut self, seq: u16) {
        self.sent.insert(seq);
    }

    /// Take one event: an ACK of one of our sends, a bare result on
    /// `channel` (a negative one is kept over a non-negative one), or a
    /// `{cmd: 1940, payload: {lock_payload, time}}` whose time is ours.
    pub fn note(&mut self, event: &Event, channel: u8, key_time: u32) {
        match event {
            Event::Acked(seq) if self.sent.contains(seq) => {
                self.acked.insert(*seq);
            }
            Event::CommandResult { code, channel: on } if *on == channel => {
                if self.result.is_none_or(|kept| kept >= 0) {
                    self.result = Some(*code);
                }
            }
            Event::Json { json, .. } if json.get("cmd").and_then(Value::as_u64) == Some(1940) => {
                let payload = &json["payload"];
                let time = payload.get("time").and_then(ff09::reply_key_time);
                if let (Some(lock_payload), Some(time)) =
                    (payload.get("lock_payload").and_then(Value::as_str), time)
                    && time == key_time
                {
                    self.reply_payload = Some(lock_payload.to_string());
                }
            }
            _ => {}
        }
    }

    /// Whether nothing more is worth waiting for: a send was delivered and
    /// the device answered.
    pub fn settled(&self) -> bool {
        !self.acked.is_empty() && (self.result.is_some() || self.reply_payload.is_some())
    }

    /// The outcome: a refusal or no delivery is an error.
    pub fn finish(self, who: &ff09::Identity, key_time: u32) -> Result<Actuation, String> {
        if let Some(code) = self.result.filter(|code| *code < 0) {
            return Err(format!("the lock refused the command (result {code})"));
        }
        if self.acked.is_empty() {
            return Err(format!(
                "the lock acknowledged none of the {} sends",
                self.sent.len()
            ));
        }
        let reply = match &self.reply_payload {
            Some(payload) => Some(ff09::parse_reply(&ff09::decrypt_frame(
                payload,
                key_time,
                &who.admin_user_id,
                &who.device_serial,
            )?)?),
            None => None,
        };
        Ok(Actuation {
            acknowledged_sends: self.acked.len(),
            result_code: self.result,
            reply,
        })
    }
}

impl Cloud {
    /// Lock (`true`) or unlock (`false`) a smart lock over P2P.
    ///
    /// Answers once the device has acknowledged delivery of the command;
    /// refuses a lock family whose wire is not ported, a device without a
    /// P2P id, and a device that answers with a negative result. Blocks for
    /// up to about a minute and a half (connect, key, sends, replies).
    pub fn set_locked(&mut self, device: &Device, locked: bool) -> Result<Actuation, String> {
        match lock_wire(device)? {
            Wire::Ff09 => {}
            Wire::VideoLockPayload => {
                return Err("the video smart lock's own command is not ported".into());
            }
            Wire::KeyedPayload => {
                return Err("the classic Wi-Fi lock's keyed command is not ported".into());
            }
        }
        if device.p2p_did.is_none() {
            return Err(format!(
                "{} has no P2P id; its lock command goes over the cloud's MQTT broker, which is not ported",
                device.serial
            ));
        }
        let who = identity_of(device, &self.acting_name())?;
        let station_serial = station_of(device);
        let standalone = station_serial == device.serial;
        let station = if standalone {
            device.clone()
        } else {
            self.devices()?
                .into_iter()
                .find(|candidate| candidate.serial == station_serial)
                .ok_or_else(|| format!("station {station_serial} is not in the device list"))?
        };
        let channel = channel_of(device, standalone)?;
        let p2p_did = station
            .p2p_did
            .clone()
            .ok_or_else(|| format!("station {station_serial} has no P2P id"))?;
        let account_id = text(&station.raw, "/member/admin_user_id")
            .map(str::to_string)
            .or_else(|| self.user_id().map(str::to_string))
            .ok_or("not signed in")?;
        let config = p2p::Config {
            serial: station_serial.clone(),
            p2p_did,
            lookup_servers: lookup_servers(&station.raw),
            dsk_key: self.dsk_key(&station_serial).ok(),
            local_address: lan_address(&station.raw),
            broadcast: true,
        };
        let mut session = Session::connect(config)?;
        let result = self.actuate(
            &mut session,
            &who,
            &account_id,
            &station_serial,
            channel,
            locked,
        );
        session.close();
        result
    }

    fn actuate(
        &mut self,
        session: &mut Session,
        who: &ff09::Identity,
        account_id: &str,
        station_serial: &str,
        channel: u8,
        locked: bool,
    ) -> Result<Actuation, String> {
        let cipher_user = account_id.to_string();
        session.establish_level2(
            &mut |cipher_id| self.cipher_keys(cipher_id, &cipher_user, station_serial),
            p2p::session::LEVEL2_GRACE,
        )?;
        let frame = ff09::lock_frame_now(locked, who)?;
        let (cmd, payload, m_value3) = transfer_command(&frame);
        let mut replies = Replies::default();
        let drain = |session: &mut Session, replies: &mut Replies| {
            while let Some(event) = session.next_event() {
                replies.note(&event, channel, frame.time);
            }
        };
        for _ in 0..SENDS {
            replies
                .sent(session.send_control_level2(cmd, channel, account_id, &payload, m_value3)?);
            session.pump(Instant::now() + SEND_SPACING)?;
            drain(session, &mut replies);
        }
        let deadline = Instant::now() + REPLY_WINDOW;
        while Instant::now() < deadline && !replies.settled() {
            session.pump(Instant::now() + Duration::from_millis(100))?;
            drain(session, &mut replies);
        }
        replies.finish(who, frame.time)
    }
}

/// The plaintext of the `SET_PAYLOAD` envelope a lock command carries,
/// before sealing (what [`Session::send_control_level2`] seals).
pub fn envelope(account_id: &str, channel: u8, frame: &ff09::Frame) -> Value {
    let (cmd, payload, m_value3) = transfer_command(frame);
    json!({"account_id": account_id, "cmd": cmd, "mChannel": channel, "mValue3": m_value3, "payload": payload})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::device_of;

    /// A synthetic stand-alone lock record shaped like a cloud listing.
    fn lock_record() -> Value {
        json!({
            "device_sn": "T8506P0000000000",
            "device_name": "Lock",
            "device_type": 184,
            "device_model": "T8506",
            "station_sn": "T8506P0000000000",
            "p2p_did": "XXXXXXX-000000-XXXXX",
            "member": {"admin_user_id": "0".repeat(40), "short_user_id": "0003"},
            "params": [
                {"param_type": 1176, "param_value": "192.168.0.20", "update_time": 10},
                {"param_type": 1177, "param_value": "10.0.0.9", "update_time": 20},
                {"param_type": 6000, "param_value": "4"}
            ]
        })
    }

    fn device(raw: Value) -> Device {
        device_of(&raw).unwrap()
    }

    #[test]
    fn the_t8506_takes_the_ff09_frame() {
        assert_eq!(lock_wire(&device(lock_record())), Ok(Wire::Ff09));
    }

    #[test]
    fn other_lock_families_take_the_wires_not_ported() {
        let mut raw = lock_record();
        raw["device_type"] = json!(55);
        assert_eq!(lock_wire(&device(raw)), Ok(Wire::VideoLockPayload));
        let mut raw = lock_record();
        raw["device_type"] = json!(51);
        raw["device_sn"] = json!("T8520P2000000000");
        assert_eq!(lock_wire(&device(raw.clone())), Ok(Wire::KeyedPayload));
        raw["device_sn"] = json!("T8520P1000000000");
        assert_eq!(lock_wire(&device(raw)), Ok(Wire::Ff09));
        let mut raw = lock_record();
        raw["device_type"] = json!(8);
        assert!(lock_wire(&device(raw)).is_err());
    }

    #[test]
    fn the_identity_comes_from_the_member_and_the_acting_name() {
        let who = identity_of(&device(lock_record()), "someone").unwrap();
        assert_eq!(who.admin_user_id, "0".repeat(40));
        assert_eq!(who.short_user_id, "0003");
        assert_eq!(who.user_name, "someone");
        assert_eq!(who.device_serial, "T8506P0000000000");
        assert!(identity_of(&device(lock_record()), "").is_err());
        let mut raw = lock_record();
        raw["member"] = json!({"admin_user_id": "0".repeat(40)});
        assert!(
            identity_of(&device(raw), "someone")
                .unwrap_err()
                .contains("short_user_id")
        );
    }

    #[test]
    fn topology_and_channel() {
        let lock = device(lock_record());
        assert_eq!(station_of(&lock), "T8506P0000000000");
        assert_eq!(channel_of(&lock, true), Ok(0));
        let mut raw = lock_record();
        raw["parent_sn"] = json!("T8010P0000000000");
        raw["station_sn"] = json!("");
        let attached = device(raw.clone());
        assert_eq!(station_of(&attached), "T8010P0000000000");
        assert!(channel_of(&attached, false).is_err());
        raw["device_channel"] = json!(3);
        assert_eq!(channel_of(&device(raw), false), Ok(3));
        let mut raw = lock_record();
        raw["parent_sn"] = json!("T8506P0000000000");
        assert_eq!(station_of(&device(raw)), "T8506P0000000000");
    }

    #[test]
    fn the_lan_address_is_the_freshest_private_one() {
        assert_eq!(
            lan_address(&lock_record()),
            Some(Ipv4Addr::new(10, 0, 0, 9))
        );
        assert_eq!(
            lan_address(&json!({"ip_addr": "192.168.1.4"})),
            Some(Ipv4Addr::new(192, 168, 1, 4))
        );
        assert_eq!(lan_address(&json!({"ip_addr": "203.0.113.4"})), None);
    }

    #[test]
    fn the_envelope_is_the_transfer_payload_in_a_set_payload() {
        let who = identity_of(&device(lock_record()), "someone").unwrap();
        let frame = ff09::lock_frame(true, &who, 1_700_000_000, 7, 1_700_000_000).unwrap();
        let value = envelope("acct", 0, &frame);
        assert_eq!(value["cmd"], 1940);
        assert_eq!(value["mChannel"], 0);
        assert_eq!(value["mValue3"], 0);
        assert_eq!(value["payload"]["apiCommand"], 6018);
        assert_eq!(value["payload"]["time"], 1_700_000_007);
        assert_eq!(value["payload"]["seq_num"], 1_700_000_000);
        assert_eq!(
            value["payload"]["lock_payload"],
            frame.lock_payload.as_str()
        );
    }

    fn who() -> ff09::Identity {
        identity_of(&device(lock_record()), "someone").unwrap()
    }

    #[test]
    fn delivery_with_no_answer_is_a_success_with_nothing_more() {
        let mut replies = Replies::default();
        replies.sent(4);
        replies.sent(5);
        replies.note(&Event::Acked(5), 0, 9);
        replies.note(&Event::Acked(77), 0, 9);
        assert!(!replies.settled());
        let outcome = replies.finish(&who(), 9).unwrap();
        assert_eq!(
            outcome,
            Actuation {
                acknowledged_sends: 1,
                result_code: None,
                reply: None
            }
        );
    }

    #[test]
    fn no_delivery_is_an_error() {
        let mut replies = Replies::default();
        replies.sent(4);
        replies.note(&Event::Acked(3), 0, 9);
        assert!(
            replies
                .finish(&who(), 9)
                .unwrap_err()
                .contains("none of the 1 sends")
        );
    }

    #[test]
    fn a_negative_result_on_the_lock_channel_is_a_refusal() {
        let mut replies = Replies::default();
        replies.sent(1);
        replies.note(&Event::Acked(1), 0, 9);
        replies.note(
            &Event::CommandResult {
                code: -5,
                channel: 3,
            },
            0,
            9,
        );
        replies.note(
            &Event::CommandResult {
                code: 0,
                channel: 0,
            },
            0,
            9,
        );
        replies.note(
            &Event::CommandResult {
                code: -108,
                channel: 0,
            },
            0,
            9,
        );
        replies.note(
            &Event::CommandResult {
                code: 0,
                channel: 0,
            },
            0,
            9,
        );
        assert!(replies.settled());
        assert!(replies.finish(&who(), 9).unwrap_err().contains("-108"));
    }

    #[test]
    fn a_reply_frame_with_our_time_is_decrypted() {
        let who = who();
        // Build the reply with the frame builder's own cipher: a frame for
        // this identity at the reply's key time decrypts to its fields.
        let sent = ff09::lock_frame(true, &who, 0x6a00_0000, 0x21, 1).unwrap();
        let mut replies = Replies::default();
        replies.sent(1);
        replies.note(&Event::Acked(1), 0, sent.time);
        let other = json!({"cmd": 1940, "payload": {"lock_payload": "ff09", "time": "1"}});
        replies.note(
            &Event::Json {
                command: 1351,
                channel: 0,
                json: other,
            },
            0,
            sent.time,
        );
        assert!(!replies.settled());
        let reply = json!({"cmd": 1940, "payload": {"dev_sn": who.device_serial, "lock_payload": sent.lock_payload, "time": format!("{:x}", sent.time)}});
        replies.note(
            &Event::Json {
                command: 1351,
                channel: 0,
                json: reply,
            },
            0,
            sent.time,
        );
        assert!(replies.settled());
        let outcome = replies.finish(&who, sent.time).unwrap();
        let parsed = outcome.reply.unwrap();
        // The frame's plaintext starts with A1 (0xa1) as the "status" byte.
        assert_eq!(parsed.status, 0xa1);
    }
}
