//! The `ff09` frame: the AES-128-CBC lock-actuation frame eufy's
//! lock-family devices take, carried over P2P inside a `TRANSFER_PAYLOAD`
//! (1940) envelope.
//!
//! **Outer frame**:
//! `ff 09 | size(u16 LE) | 03 00 02 | cmdEnc(u16 BE) | ciphertext | xor(1)`,
//! where `size` is the whole frame's length, `cmdEnc = 0x4000 | opcode`
//! (opcode 35 = lock/unlock) and `xor` is the XOR of every preceding byte.
//!
//! **Plaintext**: fields with incrementing separators `A1, A2, …`, each
//! `sep | len | bytes`, zero-padded to a 16-byte boundary before AES (which
//! then adds a full PKCS#7 block):
//!
//! ```text
//! A1 04 <unix time u32 LE>
//! A2 len <admin_user_id ASCII>      the lock owner's id
//! A3 01 <0 = lock, 1 = unlock>
//! A4 len <acting user name ASCII>   the email's local part
//! A5 len <short_user_id, hex → bytes>
//! ```
//!
//! **Cipher**: key = `admin_user_id[-12:]` (ASCII) ‖ `keyTime` (u32 BE); IV
//! = the device serial zero-padded to 16 bytes. `keyTime = unixTime | nonce`
//! with a random nonce 0..99 (the app's own randomisation); it travels in
//! the clear as the envelope's `time` so the lock can derive the key.
//!
//! Ported from mega-yfue/eufy-sdk (Apache-2.0), `src/transport/ff09.ts`,
//! whose encoder is verified byte-exact against captured T8531 video-lock
//! and T85D0 garage-door frames. The test vector below is that SDK's own
//! (`src/transport/__tests__/ff09.spec.ts`).

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use serde_json::{Value, json};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// The envelope `apiCommand` for lock/unlock (`ON_OFF_LOCK`).
pub const API_ON_OFF_LOCK: u32 = 6018;
/// The frame opcode for lock/unlock.
pub const OPCODE_ON_OFF_LOCK: u16 = 35;
/// The inner command of the P2P `SET_PAYLOAD` envelope that carries a frame.
pub const CMD_TRANSFER_PAYLOAD: u32 = 1940;

/// Who acts, and on which lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// The lock owner's account id (`member.admin_user_id`, 40 hex digits).
    pub admin_user_id: String,
    /// The acting user's display name: the login email's local part.
    pub user_name: String,
    /// The acting member's short id (`member.short_user_id`, hex, e.g. "0003").
    pub short_user_id: String,
    /// The lock's serial (the IV).
    pub device_serial: String,
}

/// A built frame and what its envelope must carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// The frame, as the hex the envelope's `lock_payload` carries.
    pub lock_payload: String,
    /// The key time, sent in the clear as the envelope's `time`.
    pub time: u32,
    /// The envelope's `seq_num`.
    pub seq_num: u32,
    /// The envelope's `apiCommand`.
    pub api_command: u32,
}

impl Frame {
    /// The `TRANSFER_PAYLOAD` payload object:
    /// `{apiCommand, lock_payload, seq_num, time}`.
    pub fn transfer_payload(&self) -> Value {
        json!({
            "apiCommand": self.api_command,
            "lock_payload": self.lock_payload,
            "seq_num": self.seq_num,
            "time": self.time,
        })
    }
}

fn check_identity(admin_user_id: &str, device_serial: &str) -> Result<(), String> {
    if admin_user_id.is_empty() {
        return Err("ff09: the lock's admin_user_id is required".into());
    }
    if admin_user_id.len() < 12 || !admin_user_id.is_ascii() {
        return Err(format!(
            "ff09: admin_user_id too short ({} < 12)",
            admin_user_id.len()
        ));
    }
    if device_serial.is_empty() {
        return Err("ff09: the device serial is required (the IV)".into());
    }
    if device_serial.len() > 16 {
        return Err(format!(
            "ff09: serial too long for the 16-byte IV ({} > 16)",
            device_serial.len()
        ));
    }
    Ok(())
}

/// Fields with incrementing separators from 0xA1. A field over 255 bytes is
/// refused rather than its length silently wrapped.
fn write_tlv(fields: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let len = u8::try_from(field.len())
            .map_err(|_| format!("ff09: field too long for a 1-byte length ({})", field.len()))?;
        out.push(0xa1 + i as u8);
        out.push(len);
        out.extend_from_slice(field);
    }
    Ok(out)
}

/// Zero-pad to a multiple of 16 (at least 16).
pub fn zero_pad(data: &[u8]) -> Vec<u8> {
    let size = data.len().max(1).div_ceil(16) * 16;
    let mut out = data.to_vec();
    out.resize(size, 0);
    out
}

fn key_and_iv(admin_user_id: &str, device_serial: &str, key_time: u32) -> ([u8; 16], [u8; 16]) {
    let mut key = [0u8; 16];
    key[..12].copy_from_slice(&admin_user_id.as_bytes()[admin_user_id.len() - 12..]);
    key[12..].copy_from_slice(&key_time.to_be_bytes());
    let mut iv = [0u8; 16];
    iv[..device_serial.len()].copy_from_slice(device_serial.as_bytes());
    (key, iv)
}

/// A random nonce 0..99 for the key time.
pub fn random_nonce() -> u32 {
    use rand_core::RngCore;
    rand_core::OsRng.next_u32() % 100
}

/// The lock/unlock frame. `unix_time`, `nonce` and `seq_num` are explicit
/// so a frame can be reproduced byte for byte; [`lock_frame_now`] fills
/// them in.
pub fn lock_frame(
    lock: bool,
    who: &Identity,
    unix_time: u32,
    nonce: u32,
    seq_num: u32,
) -> Result<Frame, String> {
    check_identity(&who.admin_user_id, &who.device_serial)?;
    if who.user_name.is_empty() || who.short_user_id.is_empty() {
        return Err("ff09: the user name and short_user_id are required".into());
    }
    if !who.user_name.is_ascii() {
        return Err("ff09: the user name must be ASCII".into());
    }
    let short = hex::decode(&who.short_user_id)
        .ok()
        .filter(|bytes| !bytes.is_empty())
        .ok_or_else(|| {
            format!(
                "ff09: short_user_id must be an even-length hex string, got {:?}",
                who.short_user_id
            )
        })?;
    let key_time = unix_time | nonce;
    let plain = zero_pad(&write_tlv(&[
        unix_time.to_le_bytes().to_vec(),
        who.admin_user_id.as_bytes().to_vec(),
        vec![if lock { 0 } else { 1 }],
        who.user_name.as_bytes().to_vec(),
        short,
    ])?);
    Ok(assemble(
        OPCODE_ON_OFF_LOCK,
        API_ON_OFF_LOCK,
        &plain,
        &who.admin_user_id,
        &who.device_serial,
        key_time,
        seq_num,
    ))
}

/// [`lock_frame`] stamped now, with a fresh nonce.
pub fn lock_frame_now(lock: bool, who: &Identity) -> Result<Frame, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as u32);
    lock_frame(lock, who, now, random_nonce(), now)
}

fn assemble(
    opcode: u16,
    api_command: u32,
    plain: &[u8],
    admin_user_id: &str,
    device_serial: &str,
    key_time: u32,
    seq_num: u32,
) -> Frame {
    let (key, iv) = key_and_iv(admin_user_id, device_serial, key_time);
    let ciphertext =
        Aes128CbcEnc::new(&key.into(), &iv.into()).encrypt_padded_vec_mut::<Pkcs7>(plain);
    let size = 2 + 2 + 3 + 2 + ciphertext.len() + 1;
    let mut frame = Vec::with_capacity(size);
    frame.extend_from_slice(&[0xff, 0x09]);
    frame.extend_from_slice(&(size as u16).to_le_bytes());
    frame.extend_from_slice(&[0x03, 0x00, 0x02]);
    frame.extend_from_slice(&(0x4000 | opcode).to_be_bytes());
    frame.extend_from_slice(&ciphertext);
    let xor = frame.iter().fold(0u8, |acc, byte| acc ^ byte);
    frame.push(xor);
    Frame {
        lock_payload: hex::encode(frame),
        time: key_time,
        seq_num,
        api_command,
    }
}

/// Decrypt a frame the device sent back (same key and IV derivation, keyed
/// by the `time` its envelope carries), after checking its magic, size and
/// checksum. Answers the plaintext with the PKCS#7 padding removed.
pub fn decrypt_frame(
    lock_payload: &str,
    key_time: u32,
    admin_user_id: &str,
    device_serial: &str,
) -> Result<Vec<u8>, String> {
    check_identity(admin_user_id, device_serial)?;
    let frame = hex::decode(lock_payload).map_err(|_| "ff09: a frame that is not hex")?;
    if frame.len() < 10 || frame[0] != 0xff || frame[1] != 0x09 {
        return Err("ff09: frame missing the ff09 magic".into());
    }
    let size = u16::from_le_bytes([frame[2], frame[3]]) as usize;
    if size != frame.len() {
        return Err(format!(
            "ff09: size mismatch (header says {size}, got {})",
            frame.len()
        ));
    }
    let (body, xor) = frame.split_at(frame.len() - 1);
    if body.iter().fold(0u8, |acc, byte| acc ^ byte) != xor[0] {
        return Err("ff09: frame failed its XOR checksum".into());
    }
    let (key, iv) = key_and_iv(admin_user_id, device_serial, key_time);
    Aes128CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(&body[9..])
        .map_err(|_| "ff09: frame did not decrypt".to_string())
}

/// A frame's opcode (`cmdEnc` with the flag bits masked off) and its flag
/// bits, or `None` for something too short to be a frame.
pub fn frame_opcode(lock_payload: &str) -> Option<(u16, u16)> {
    let frame = hex::decode(lock_payload).ok()?;
    let enc = u16::from_be_bytes([*frame.get(7)?, *frame.get(8)?]);
    Some((enc & 0x00ff, enc & 0xff00))
}

/// A device reply's plaintext: its leading status byte, then fields keyed
/// by separator (the reply's own numbering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u8,
    pub fields: Vec<(u8, Vec<u8>)>,
}

/// Read a decrypted reply. The walk stops at a zero separator (padding) or
/// a field running past the end.
pub fn parse_reply(plain: &[u8]) -> Result<Reply, String> {
    let (&status, rest) = plain
        .split_first()
        .ok_or("ff09: reply too short (no status byte)")?;
    let mut fields = Vec::new();
    let mut i = 0;
    while i + 2 <= rest.len() {
        let (tag, len) = (rest[i], rest[i + 1] as usize);
        if tag == 0 || i + 2 + len > rest.len() {
            break;
        }
        fields.push((tag, rest[i + 2..i + 2 + len].to_vec()));
        i += 2 + len;
    }
    Ok(Reply { status, fields })
}

/// The key time a device reply's envelope carries: a hex string on the
/// replies seen, a number otherwise.
pub fn reply_key_time(time: &Value) -> Option<u32> {
    match time {
        Value::Number(number) => number.as_u64().map(|value| value as u32),
        Value::String(text) => u32::from_str_radix(text.trim(), 16).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SDK's vector: a captured T8531 unlock frame, re-derived to a
    /// synthetic identity and reproduced from it byte for byte.
    fn identity() -> Identity {
        Identity {
            admin_user_id: "0".repeat(40),
            user_name: "someone+tag".into(),
            short_user_id: "0003".into(),
            device_serial: "T8531K0000000000".into(),
        }
    }
    const UNIX_TIME: u32 = 1783612332;
    const SEQ_NUM: u32 = 1783612326;
    const CAPTURED: &str = concat!(
        "ff096a0003000240230c1fa86e364a2a504c67567748f93d3bfff5318a8baadadddbf6909827400b12525531a0",
        "5628097cf1c49e9928b5a0f2d9e11648e42aabc55432c799248c26d4ab2339a65c622e5119f4689982b80b95b0c",
        "5554d07385bc3064b8a5f2cbb87b387"
    );

    #[test]
    fn reproduces_the_captured_unlock_frame() {
        let frame = lock_frame(false, &identity(), UNIX_TIME, 1, SEQ_NUM).unwrap();
        assert_eq!(frame.lock_payload, CAPTURED);
        assert_eq!(frame.time, 1783612333);
        assert_eq!(frame.seq_num, SEQ_NUM);
        assert_eq!(frame.api_command, API_ON_OFF_LOCK);
    }

    #[test]
    fn lock_and_unlock_differ_only_in_the_a3_byte() {
        let lock = lock_frame(true, &identity(), UNIX_TIME, 1, SEQ_NUM).unwrap();
        let unlock = lock_frame(false, &identity(), UNIX_TIME, 1, SEQ_NUM).unwrap();
        assert_ne!(lock.lock_payload, unlock.lock_payload);
        assert_eq!(&lock.lock_payload[14..18], "4023");
        let who = identity();
        let plain = |frame: &Frame| {
            decrypt_frame(
                &frame.lock_payload,
                frame.time,
                &who.admin_user_id,
                &who.device_serial,
            )
            .unwrap()
        };
        let (a, b) = (plain(&lock), plain(&unlock));
        let differing: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
        // A1(6) + A2(2+40) = 48, then A3's separator, length, value.
        assert_eq!(differing, vec![50]);
        assert_eq!((a[48], a[49], a[50]), (0xa3, 1, 0));
        assert_eq!(b[50], 1);
    }

    #[test]
    fn a_frame_carries_its_magic_size_and_checksum() {
        let frame = lock_frame_now(true, &identity()).unwrap();
        let bytes = hex::decode(&frame.lock_payload).unwrap();
        assert_eq!(&bytes[..2], &[0xff, 0x09]);
        assert_eq!(
            u16::from_le_bytes([bytes[2], bytes[3]]) as usize,
            bytes.len()
        );
        let xor = bytes[..bytes.len() - 1].iter().fold(0, |acc, b| acc ^ b);
        assert_eq!(bytes[bytes.len() - 1], xor);
        assert_eq!(frame_opcode(&frame.lock_payload), Some((35, 0x4000)));
        assert!(frame.time & !127 == frame.seq_num & !127);
    }

    #[test]
    fn the_plaintext_is_the_documented_fields_zero_padded() {
        let who = identity();
        let frame = lock_frame(false, &who, UNIX_TIME, 1, SEQ_NUM).unwrap();
        let plain = decrypt_frame(
            &frame.lock_payload,
            frame.time,
            &who.admin_user_id,
            &who.device_serial,
        )
        .unwrap();
        let mut expected = vec![0xa1, 4];
        expected.extend_from_slice(&UNIX_TIME.to_le_bytes());
        expected.extend_from_slice(&[0xa2, 40]);
        expected.extend_from_slice(who.admin_user_id.as_bytes());
        expected.extend_from_slice(&[0xa3, 1, 1, 0xa4, 11]);
        expected.extend_from_slice(b"someone+tag");
        expected.extend_from_slice(&[0xa5, 2, 0x00, 0x03]);
        assert_eq!(plain, zero_pad(&expected));
        assert_eq!(plain.len() % 16, 0);
    }

    #[test]
    fn the_transfer_payload_names_the_envelope_fields() {
        let frame = lock_frame(true, &identity(), UNIX_TIME, 1, SEQ_NUM).unwrap();
        let payload = frame.transfer_payload();
        assert_eq!(payload["apiCommand"], 6018);
        assert_eq!(payload["lock_payload"], frame.lock_payload.as_str());
        assert_eq!(payload["seq_num"], SEQ_NUM);
        assert_eq!(payload["time"], 1783612333);
    }

    #[test]
    fn missing_or_malformed_identity_is_refused() {
        let mut who = identity();
        who.admin_user_id.clear();
        assert!(
            lock_frame(true, &who, 1, 0, 1)
                .unwrap_err()
                .contains("admin_user_id")
        );
        let mut who = identity();
        who.device_serial.clear();
        assert!(
            lock_frame(true, &who, 1, 0, 1)
                .unwrap_err()
                .contains("serial")
        );
        let mut who = identity();
        who.device_serial = "T8531K00000000001".into();
        assert!(
            lock_frame(true, &who, 1, 0, 1)
                .unwrap_err()
                .contains("too long")
        );
        let mut who = identity();
        who.user_name = "x".repeat(256);
        assert!(
            lock_frame(true, &who, 1, 0, 1)
                .unwrap_err()
                .contains("too long")
        );
        for bad in ["003", "zz03"] {
            let mut who = identity();
            who.short_user_id = bad.into();
            assert!(
                lock_frame(true, &who, 1, 0, 1)
                    .unwrap_err()
                    .contains("even-length hex")
            );
        }
    }

    /// A reply built the way the SDK's test fixture builds a device's
    /// settings reply (`ff09-test-fixtures.ts`): flag bits 0x4800.
    fn device_reply(plain: &[u8], key_time: u32, who: &Identity) -> String {
        let (key, iv) = key_and_iv(&who.admin_user_id, &who.device_serial, key_time);
        let ciphertext =
            Aes128CbcEnc::new(&key.into(), &iv.into()).encrypt_padded_vec_mut::<Pkcs7>(plain);
        let mut frame = vec![0xff, 0x09, 0, 0, 0x03, 0x00, 0x02, 0x48, 0x35];
        frame.extend_from_slice(&ciphertext);
        let size = (frame.len() + 1) as u16;
        frame[2..4].copy_from_slice(&size.to_le_bytes());
        let xor = frame.iter().fold(0, |acc, b| acc ^ b);
        frame.push(xor);
        hex::encode(frame)
    }

    #[test]
    fn a_device_reply_decrypts_and_parses() {
        let who = identity();
        let plain = [0x00, 0xa1, 1, 1, 0xa2, 2, 0x5a, 0x00, 0, 0, 0];
        let reply = device_reply(&plain, 0x6a4f_31ad, &who);
        let opened =
            decrypt_frame(&reply, 0x6a4f_31ad, &who.admin_user_id, &who.device_serial).unwrap();
        assert_eq!(opened, plain);
        let parsed = parse_reply(&opened).unwrap();
        assert_eq!(parsed.status, 0);
        assert_eq!(parsed.fields, vec![(0xa1, vec![1]), (0xa2, vec![0x5a, 0])]);
        assert_eq!(frame_opcode(&reply), Some((0x35, 0x4800)));
    }

    #[test]
    fn a_damaged_reply_is_refused() {
        let who = identity();
        let reply = device_reply(&[0x00], 7, &who);
        let mut bytes = hex::decode(&reply).unwrap();
        bytes[12] ^= 1;
        let damaged = hex::encode(&bytes);
        assert!(
            decrypt_frame(&damaged, 7, &who.admin_user_id, &who.device_serial)
                .unwrap_err()
                .contains("checksum")
        );
        assert!(
            decrypt_frame(
                &reply[..reply.len() - 2],
                7,
                &who.admin_user_id,
                &who.device_serial
            )
            .unwrap_err()
            .contains("size")
        );
        assert!(decrypt_frame("0000", 7, &who.admin_user_id, &who.device_serial).is_err());
        assert!(parse_reply(&[]).is_err());
    }

    #[test]
    fn a_reply_time_is_read_as_hex_text_or_a_number() {
        assert_eq!(reply_key_time(&json!("6a4f31ad")), Some(0x6a4f_31ad));
        assert_eq!(reply_key_time(&json!(1783612333u32)), Some(1783612333));
        assert_eq!(reply_key_time(&json!("zz")), None);
        assert_eq!(reply_key_time(&json!(null)), None);
    }
}
