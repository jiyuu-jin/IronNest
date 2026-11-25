//! The pure pieces of eufy's P2P protocol (ThroughTek PPCS): packet and
//! frame layouts, lookup and relay payloads, and the level-1/level-2
//! ciphers. Nothing here touches a socket.
//!
//! Every UDP packet is `[type:2][payload length:2 BE][payload]`; the first
//! type byte is always 0xf1. A DATA packet carries
//! `[d1 <data type>][seq:2 BE]` and then frames, each starting with the
//! ASCII magic `XZYH` and a 16-byte header.
//!
//! Ported from mega-yfue/eufy-sdk (Apache-2.0),
//! `src/transport/p2p/codec.ts`, which documents these from captures of
//! the vendor app and its disassembled native library.

use std::net::{Ipv4Addr, SocketAddrV4};

use aes::Aes128;
use aes::cipher::{BlockDecrypt, BlockDecryptMut, BlockEncrypt, KeyInit, KeyIvInit};
use aes_gcm::{Aes256Gcm, Nonce, Tag, aead::AeadInPlace};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// The port a device answers local lookups on.
pub const LOCAL_LOOKUP_PORT: u16 = 32108;
/// The port of the cloud lookup servers a `p2p_conn` names.
pub const CLOUD_LOOKUP_PORT: u16 = 32100;
/// The magic at the start of every data frame.
pub const MAGIC: &[u8; 4] = b"XZYH";
/// A data frame's header length, magic included.
pub const FRAME_HEADER_BYTES: usize = 16;
/// The AAD of every level-2 (AES-256-GCM) frame.
pub const GCM_AAD: &[u8] = b"eufy security";
/// The level-2 sub-header counter, read as one u32 LE from `00 03 02 01`.
pub const LEVEL2_SEQ_BASE: u32 = 0x0102_0300;

/// Packet types this client sends.
pub mod request {
    pub const LOCAL_LOOKUP: [u8; 2] = [0xf1, 0x30];
    pub const LOOKUP_WITH_KEY: [u8; 2] = [0xf1, 0x26];
    /// `ListReq_DSK`: the variant-2 lookup, answered with a relay list.
    pub const LOOKUP_WITH_KEY2: [u8; 2] = [0xf1, 0x6a];
    pub const RLY_HELLO: [u8; 2] = [0xf1, 0x70];
    pub const RLY_PORT: [u8; 2] = [0xf1, 0x72];
    pub const RLY_REQ: [u8; 2] = [0xf1, 0x80];
    pub const RLY_PKT: [u8; 2] = [0xf1, 0x83];
    pub const CHECK_CAM: [u8; 2] = [0xf1, 0x41];
    pub const PING: [u8; 2] = [0xf1, 0xe0];
    pub const PONG: [u8; 2] = [0xf1, 0xe1];
    pub const DATA: [u8; 2] = [0xf1, 0xd0];
    pub const ACK: [u8; 2] = [0xf1, 0xd1];
    pub const END: [u8; 2] = [0xf1, 0xf0];
}

/// Packet types this client receives.
pub mod response {
    pub const LOCAL_LOOKUP_RESP: [u8; 2] = [0xf1, 0x41];
    pub const LOOKUP_ADDR: [u8; 2] = [0xf1, 0x40];
    /// The relays a lookup server names: `[count:1][pad:3]`, then records.
    pub const LIST_REQ_ACK: [u8; 2] = [0xf1, 0x69];
    pub const RLY_HELLO_ACK: [u8; 2] = [0xf1, 0x71];
    /// `[mark:4 BE][port:2 BE][pad:2]`.
    pub const RLY_PORT_ACK: [u8; 2] = [0xf1, 0x73];
    pub const RLY_REQ_ACK: [u8; 2] = [0xf1, 0x81];
    /// A relay endpoint to register with: a record, then `[handle:4 BE]`.
    pub const RLY_TO: [u8; 2] = [0xf1, 0x82];
    pub const CAM_ID: [u8; 2] = [0xf1, 0x42];
    /// A lookup server accepting the P2P request when its first byte is 0.
    pub const P2P_REQ_ACK: [u8; 2] = [0xf1, 0x21];
    pub const CAM_ADDR: [u8; 2] = [0xf1, 0x43];
    /// The relay reporting the path ready: the device's 20-byte id.
    pub const RLY_RDY: [u8; 2] = [0xf1, 0x84];
    pub const PING: [u8; 2] = [0xf1, 0xe0];
    pub const PONG: [u8; 2] = [0xf1, 0xe1];
    pub const DATA: [u8; 2] = [0xf1, 0xd0];
    pub const ACK: [u8; 2] = [0xf1, 0xd1];
    pub const END: [u8; 2] = [0xf1, 0xf0];
}

/// Data channels (the second byte of a DATA packet's type header).
pub mod data_type {
    pub const DATA: u8 = 0;
    pub const VIDEO: u8 = 1;
    pub const CONTROL: u8 = 2;
    pub const BINARY: u8 = 3;
}

/// `[type:2][payload length:2 BE][payload]`.
pub fn frame_message(kind: [u8; 2], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&kind);
    out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Whether `msg` is of packet type `kind`.
pub fn has_header(msg: &[u8], kind: [u8; 2]) -> bool {
    msg.len() >= 2 && msg[0] == kind[0] && msg[1] == kind[1]
}

/// A P2P id (`ABCDEFG-000000-ABCDE`) as the 20 bytes lookups carry:
/// prefix in 8, the number u32 BE, suffix in 8.
pub fn p2p_did_bytes(p2p_did: &str) -> Result<[u8; 20], String> {
    let parts: Vec<&str> = p2p_did.split('-').collect();
    let [prefix, number, suffix] = parts[..] else {
        return Err("a P2P id that is not PREFIX-NUMBER-SUFFIX".into());
    };
    let number: u32 = number
        .parse()
        .map_err(|_| "a P2P id whose middle is not a number".to_string())?;
    if prefix.len() > 8 || suffix.len() > 8 {
        return Err("a P2P id with a part over 8 characters".into());
    }
    let mut out = [0u8; 20];
    out[..prefix.len()].copy_from_slice(prefix.as_bytes());
    out[8..12].copy_from_slice(&number.to_be_bytes());
    out[12..12 + suffix.len()].copy_from_slice(suffix.as_bytes());
    Ok(out)
}

const CLOUD_IP_TABLE: [u8; 54] = [
    0x49, 0x59, 0x43, 0x3d, 0xb5, 0xbf, 0x6d, 0xa3, 0x47, 0x53, 0x4f, 0x61, 0x65, 0xe3, 0x71, 0xe9,
    0x67, 0x7f, 0x02, 0x03, 0x0b, 0xad, 0xb3, 0x89, 0x2b, 0x2f, 0x35, 0xc1, 0x6b, 0x8b, 0x95, 0x97,
    0x11, 0xe5, 0xa7, 0x0d, 0xef, 0xf1, 0x05, 0x07, 0x83, 0xfb, 0x9d, 0x3b, 0xc5, 0xc7, 0x13, 0x17,
    0x1d, 0x1f, 0x25, 0x29, 0xd3, 0xdf,
];

/// The cloud lookup servers a device record's `p2p_conn` (or `app_conn`)
/// names: letter pairs XOR-chained under a fixed table, seed 0x39, decoding
/// to comma-separated hosts.
pub fn decode_cloud_hosts(conn: &str) -> Vec<String> {
    let encoded = conn.split(':').next().unwrap_or_default().as_bytes();
    let mut output = vec![0u8; encoded.len() / 2];
    for i in 0..output.len() {
        let z = output[..i].iter().fold(0x39u8, |acc, byte| acc ^ byte);
        let high = encoded[i * 2].wrapping_sub(b'A') << 4;
        let low = encoded[i * 2 + 1].wrapping_sub(b'A');
        output[i] = z ^ CLOUD_IP_TABLE[i % CLOUD_IP_TABLE.len()] ^ high.wrapping_add(low);
    }
    String::from_utf8_lossy(&output)
        .split(',')
        .map(|host| host.trim_matches(char::from(0)).trim().to_string())
        .filter(|host| !host.is_empty())
        .collect()
}

/// The level-1 AES-128-ECB key: the serial's last 7 characters, then the
/// 9 characters of the P2P id from its first dash.
pub fn level1_key(serial: &str, p2p_did: &str) -> Result<[u8; 16], String> {
    let dash = p2p_did.find('-').ok_or("a P2P id with no dash")?;
    let tail = serial
        .get(serial.len().saturating_sub(7)..)
        .ok_or("a serial that is not ASCII")?;
    let text = format!("{tail}{}", p2p_did.get(dash..dash + 9).unwrap_or_default());
    text.as_bytes()
        .try_into()
        .map_err(|_| format!("a level-1 key of {} bytes, not 16", text.len()))
}

/// Zero-pad to a multiple of 16 (eufy pads with zeros, not PKCS#7).
pub fn zero_pad(data: &[u8]) -> Vec<u8> {
    crate::ff09::zero_pad(data)
}

/// AES-128-ECB without padding; `data` must be whole blocks.
pub fn ecb_encrypt(data: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes128::new(key.into());
    let mut out = data.to_vec();
    for block in out.chunks_exact_mut(16) {
        cipher.encrypt_block(block.into());
    }
    out
}

/// The inverse of [`ecb_encrypt`]; `None` unless `data` is whole blocks.
pub fn ecb_decrypt(data: &[u8], key: &[u8; 16]) -> Option<Vec<u8>> {
    if !data.len().is_multiple_of(16) {
        return None;
    }
    let cipher = Aes128::new(key.into());
    let mut out = data.to_vec();
    for block in out.chunks_exact_mut(16) {
        cipher.decrypt_block(block.into());
    }
    Some(out)
}

/// A level-2 command body (sign code 8):
/// `tag(16) ‖ nonce(12) ‖ seq(4 LE) ‖ ciphertext`, AES-256-GCM, AAD
/// "eufy security"; `seq` is [`LEVEL2_SEQ_BASE`] plus the send count.
pub fn seal_level2(key: &[u8; 32], nonce: [u8; 12], counter: u32, plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(key.into());
    let mut buffer = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(Nonce::from_slice(&nonce), GCM_AAD, &mut buffer)
        .expect("a plaintext within GCM's limit");
    let mut out = Vec::with_capacity(32 + buffer.len());
    out.extend_from_slice(&tag);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&LEVEL2_SEQ_BASE.wrapping_add(counter).to_le_bytes());
    out.extend_from_slice(&buffer);
    out
}

/// Open a level-2 frame: sign code 8 carries the 4-byte sub-header (ciphertext
/// at 32), sign code 2 does not (at 28). `None` when it does not authenticate.
pub fn open_level2(key: &[u8; 32], payload: &[u8], sign_code: u8) -> Option<Vec<u8>> {
    let at = if sign_code == 8 { 32 } else { 28 };
    if payload.len() < at {
        return None;
    }
    let cipher = Aes256Gcm::new(key.into());
    let mut buffer = payload[at..].to_vec();
    cipher
        .decrypt_in_place_detached(
            Nonce::from_slice(&payload[16..28]),
            GCM_AAD,
            &mut buffer,
            Tag::from_slice(&payload[..16]),
        )
        .ok()?;
    Some(buffer)
}

/// The `cipher_id` a decrypted `CMD_GATEWAYINFO` reply names (u16 LE at 0).
pub fn gateway_cipher_id(payload: &[u8]) -> u16 {
    payload
        .get(..2)
        .map_or(0, |bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// The shortest `CMD_GATEWAYINFO` reply that can hold a key envelope:
/// `cipher_id(2) ‖ 00 00` and a 512-bit RSA ciphertext, the smallest RSA
/// key anything uses (eufy's are 1024-bit; the ECC envelope is
/// `4 + 33 + 16 + 48` = 101 bytes).
pub const SHORTEST_KEY_ENVELOPE: usize = 4 + 64;

/// Whether a decrypted `CMD_GATEWAYINFO` reply is too short to hold a key
/// envelope: the device offers no level-2 key. The Smart Lock C220
/// (T8506) answers with a result code (-103, invalid command) and
/// whatever padding its seal leaves; read as an envelope, its first two
/// bytes name a cipher (0xff99 = 65433) the cloud has no keys for.
pub fn gateway_holds_no_key(payload: &[u8]) -> bool {
    payload.len() < SHORTEST_KEY_ENVELOPE
}

/// eufy's ECIES KDF: HMAC-SHA256 feedback under the label "ECIES"; each
/// 32-byte block is `HMAC(S, t ‖ "ECIES")` with `t = HMAC(S, t)` first.
pub fn eufy_kdf(shared: &[u8], len: usize) -> Vec<u8> {
    let hmac = |data: &[&[u8]]| {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(shared).expect("any key length");
        for part in data {
            mac.update(part);
        }
        mac.finalize().into_bytes().to_vec()
    };
    let mut out = Vec::new();
    let mut t = b"ECIES".to_vec();
    while out.len() < len {
        t = hmac(&[t.as_slice()]);
        out.extend(hmac(&[t.as_slice(), b"ECIES".as_slice()]));
    }
    out.truncate(len);
    out
}

/// The level-2 AES-256-GCM key out of a decrypted `CMD_GATEWAYINFO` reply,
/// opened with the cipher's ECC private key (`ecc_private_key`, hex):
/// `cipher_id(2) ‖ 00 00 ‖ ephemeralPub(33, compressed) ‖ iv(16) ‖ ct(48)`;
/// `S = ECDH_X(priv, ephemeralPub)`, `aesKey = kdf(S)[0..16]`, and the
/// AES-128-CBC plaintext's first 32 bytes are the key.
pub fn level2_key_from_gateway_ecc(payload: &[u8], ecc_private_hex: &str) -> Option<[u8; 32]> {
    let envelope = payload.get(4..4 + 33 + 16 + 48)?;
    let secret = p256::SecretKey::from_slice(&hex::decode(ecc_private_hex.trim()).ok()?).ok()?;
    let ephemeral = p256::PublicKey::from_sec1_bytes(&envelope[..33]).ok()?;
    let shared = p256::ecdh::diffie_hellman(secret.to_nonzero_scalar(), ephemeral.as_affine());
    let kdf = eufy_kdf(shared.raw_secret_bytes(), 48);
    let key: [u8; 16] = kdf[..16].try_into().ok()?;
    let iv: [u8; 16] = envelope[33..49].try_into().ok()?;
    let mut ct = envelope[49..97].to_vec();
    let mut decryptor = cbc::Decryptor::<Aes128>::new(&key.into(), &iv.into());
    for block in ct.chunks_exact_mut(16) {
        decryptor.decrypt_block_mut(block.into());
    }
    ct[..32].try_into().ok()
}

/// The level-2 AES-128-ECB key out of a decrypted `CMD_GATEWAYINFO` reply
/// for a cipher with no ECC key, opened with its RSA key (`private_key`,
/// base64 PKCS#8 DER): `cipher_id(2) ‖ 00 00 ‖ RSA ciphertext`, the
/// 16-byte key under PKCS#1 v1.5 padding. The padding is checked over the
/// raw RSA result, so a wrong key fails closed.
pub fn level2_key_from_gateway_rsa(payload: &[u8], rsa_private_b64: &str) -> Option<[u8; 16]> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::traits::PublicKeyParts;
    let der = STANDARD.decode(rsa_private_b64.trim()).ok()?;
    let key = rsa::RsaPrivateKey::from_pkcs8_der(&der).ok()?;
    let len = key.size();
    let ct = payload.get(4..4 + len)?;
    let block = rsa::hazmat::rsa_decrypt_and_check(
        &key,
        Some(&mut rand_core::OsRng),
        &rsa::BigUint::from_bytes_be(ct),
    )
    .ok()?
    .to_bytes_be();
    let mut padded = vec![0u8; len.checked_sub(block.len())?];
    padded.extend(block);
    if padded[0] != 0 || padded[1] != 2 {
        return None;
    }
    let separator = 2 + padded[2..].iter().position(|&byte| byte == 0)?;
    if separator < 10 {
        return None;
    }
    padded[separator + 1..].try_into().ok()
}

/// A local lookup's payload.
pub fn local_lookup_payload() -> [u8; 2] {
    [0, 0]
}

/// An address record: `[0x0002 BE][port LE][IPv4 reversed][8 zero bytes]`.
pub fn encode_address(addr: SocketAddrV4) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[..2].copy_from_slice(&2u16.to_be_bytes());
    out[2..4].copy_from_slice(&addr.port().to_le_bytes());
    let octets = addr.ip().octets();
    out[4..8].copy_from_slice(&[octets[3], octets[2], octets[1], octets[0]]);
    out
}

/// The address record at `offset`: port LE at +2, IPv4 reversed at +4.
pub fn read_address(buf: &[u8], offset: usize) -> Option<SocketAddrV4> {
    let record = buf.get(offset..offset + 8)?;
    Some(SocketAddrV4::new(
        Ipv4Addr::new(record[7], record[6], record[5], record[4]),
        u16::from_le_bytes([record[2], record[3]]),
    ))
}

/// The variant-2 cloud lookup: `[p2p id:20][dsk key][00 00 00 00]`.
pub fn lookup_with_key2_payload(did: &[u8; 20], dsk_key: &str) -> Vec<u8> {
    let mut out = did.to_vec();
    out.extend_from_slice(dsk_key.as_bytes());
    out.extend_from_slice(&[0; 4]);
    out
}

/// The classic cloud lookup the app sends:
/// `[p2p id:20][own address record:16][02 05 01 05][dsk key][00 00 00 00]`.
pub fn lookup_with_key_payload(did: &[u8; 20], own: SocketAddrV4, dsk_key: &str) -> Vec<u8> {
    let mut out = did.to_vec();
    out.extend_from_slice(&encode_address(own));
    out.extend_from_slice(&[0x02, 0x05, 0x01, 0x05]);
    out.extend_from_slice(dsk_key.as_bytes());
    out.extend_from_slice(&[0; 4]);
    out
}

/// `RLY_REQ`: `[p2p id:20][relay record:16][mark:4 BE]`.
pub fn relay_request_payload(did: &[u8; 20], relay: SocketAddrV4, mark: u32) -> Vec<u8> {
    let mut out = did.to_vec();
    out.extend_from_slice(&encode_address(relay));
    out.extend_from_slice(&mark.to_be_bytes());
    out
}

/// `RLY_PKT`: `[handle:4 BE][p2p id:20][role 0][pad:3]`.
pub fn relay_packet_payload(handle: u32, did: &[u8; 20]) -> Vec<u8> {
    let mut out = handle.to_be_bytes().to_vec();
    out.extend_from_slice(did);
    out.extend_from_slice(&[0; 4]);
    out
}

/// `CHECK_CAM` (the hole punch): `[p2p id:20][00 00 00]`.
pub fn check_cam_payload(did: &[u8; 20]) -> Vec<u8> {
    let mut out = did.to_vec();
    out.extend_from_slice(&[0; 3]);
    out
}

/// The relays a `LIST_REQ_ACK` names, in order, without unspecified ones.
pub fn parse_relay_list(msg: &[u8]) -> Vec<SocketAddrV4> {
    let count = msg.get(4).copied().unwrap_or(0) as usize;
    (0..count)
        .take_while(|i| 8 + 16 * (i + 1) <= msg.len())
        .filter_map(|i| read_address(msg, 8 + 16 * i))
        .filter(|addr| !addr.ip().is_unspecified())
        .collect()
}

/// A `RLY_PORT_ACK`'s mark and relay port.
pub fn parse_relay_port_ack(msg: &[u8]) -> Option<(u32, u16)> {
    (msg.len() >= 10).then(|| {
        (
            u32::from_be_bytes(msg[4..8].try_into().expect("4 bytes")),
            u16::from_be_bytes([msg[8], msg[9]]),
        )
    })
}

/// A `RLY_TO`'s relay endpoint and handle.
pub fn parse_relay_to(msg: &[u8]) -> Option<(SocketAddrV4, u32)> {
    (msg.len() >= 24).then(|| {
        (
            read_address(msg, 4).expect("in range"),
            u32::from_be_bytes(msg[20..24].try_into().expect("4 bytes")),
        )
    })
}

/// The 10-byte head of an outbound DATA payload:
/// `[d1 00][seq:2 BE]["XZYH"][command:2 LE]`.
pub fn command_header(seq: u16, command: u16) -> [u8; 10] {
    let mut out = [0u8; 10];
    out[..2].copy_from_slice(&[0xd1, data_type::DATA]);
    out[2..4].copy_from_slice(&seq.to_be_bytes());
    out[4..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&command.to_le_bytes());
    out
}

/// The rest of a frame header around `data`:
/// `len(2 LE) ‖ 00 00 ‖ magic(2) ‖ [channel, sign code] ‖ [stream, 0] ‖ data`.
pub fn command_body(data: &[u8], channel: u8, sign_code: u8, magic: [u8; 2]) -> Vec<u8> {
    let mut out = Vec::with_capacity(10 + data.len());
    out.extend_from_slice(&(data.len() as u16).to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&magic);
    out.extend_from_slice(&[channel, sign_code, 0, 0]);
    out.extend_from_slice(data);
    out
}

/// An argument-less command's body on `channel` (e.g. `CMD_GATEWAYINFO`).
pub fn void_body(channel: u8) -> Vec<u8> {
    command_body(&[], channel, 0, [0x01, 0x00])
}

/// The ACK of a received DATA packet: `[d1 <type>][count 1:2 BE][seq:2 BE]`.
pub fn ack_payload(data_type: u8, seq: u16) -> [u8; 6] {
    let mut out = [0xd1, data_type, 0, 1, 0, 0];
    out[4..].copy_from_slice(&seq.to_be_bytes());
    out
}

/// The sequence numbers an ACK packet acknowledges on `data_type`.
pub fn parse_ack(msg: &[u8], wanted_type: u8) -> Vec<u16> {
    if msg.len() < 8 || msg[4] != 0xd1 || msg[5] != wanted_type {
        return Vec::new();
    }
    let count = u16::from_be_bytes([msg[6], msg[7]]) as usize;
    (0..count)
        .take_while(|i| 10 + 2 * i <= msg.len())
        .map(|i| u16::from_be_bytes([msg[8 + 2 * i], msg[9 + 2 * i]]))
        .collect()
}

/// A data frame's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub command: u16,
    pub length: u32,
    pub channel: u8,
    pub sign_code: u8,
    pub kind: u8,
}

/// Read the 16-byte header starting at the magic.
pub fn parse_frame_header(frame: &[u8]) -> Option<FrameHeader> {
    if frame.len() < FRAME_HEADER_BYTES || &frame[..4] != MAGIC {
        return None;
    }
    Some(FrameHeader {
        command: u16::from_le_bytes([frame[4], frame[5]]),
        length: u32::from_le_bytes(frame[6..10].try_into().expect("4 bytes")),
        channel: frame[12],
        sign_code: frame[13],
        kind: frame[14],
    })
}

/// The text up to the first NUL.
pub fn nul_terminated(data: &[u8]) -> String {
    let end = data
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DID: &str = "EUPRCAM-000000-XXXXX";

    /// The inverse of [`decode_cloud_hosts`], as the SDK's spec writes it.
    fn encode_cloud_hosts(plain: &str) -> String {
        let bytes = plain.as_bytes();
        let mut out = String::new();
        for i in 0..bytes.len() {
            let z = bytes[..i].iter().fold(0x39u8, |acc, b| acc ^ b);
            let b = bytes[i] ^ z ^ CLOUD_IP_TABLE[i % CLOUD_IP_TABLE.len()];
            out.push((b'A' + (b >> 4)) as char);
            out.push((b'A' + (b & 0x0f)) as char);
        }
        out
    }

    #[test]
    fn cloud_hosts_round_trip() {
        let conn = encode_cloud_hosts("18.197.1.2,3.69.4.5");
        assert_eq!(decode_cloud_hosts(&conn), vec!["18.197.1.2", "3.69.4.5"]);
        assert_eq!(
            decode_cloud_hosts(&format!("{conn}:extra")),
            vec!["18.197.1.2", "3.69.4.5"]
        );
        assert!(decode_cloud_hosts("").is_empty());
    }

    #[test]
    fn the_level1_key_comes_from_serial_and_p2p_id() {
        let key = level1_key("T8030T0000000000", DID).unwrap();
        assert_eq!(&key, b"0000000-000000-X");
        assert!(level1_key("T8030T0000000000", "nodash").is_err());
    }

    #[test]
    fn a_p2p_id_packs_into_20_bytes() {
        let bytes = p2p_did_bytes(DID).unwrap();
        assert_eq!(&bytes[..7], b"EUPRCAM");
        assert_eq!(bytes[7], 0);
        assert_eq!(&bytes[8..12], &[0, 0, 0, 0]);
        assert_eq!(&bytes[12..17], b"XXXXX");
        assert_eq!(p2p_did_bytes("ABC-000258-XY").unwrap()[8..12], [0, 0, 1, 2]);
        assert!(p2p_did_bytes("ABC-XY").is_err());
        assert!(p2p_did_bytes("ABC-nope-XY").is_err());
    }

    #[test]
    fn ecb_round_trips_whole_blocks_only() {
        let key = *b"0123456789abcdef";
        let plain = [7u8; 32];
        let sealed = ecb_encrypt(&plain, &key);
        assert_ne!(sealed, plain);
        assert_eq!(ecb_decrypt(&sealed, &key).unwrap(), plain);
        assert!(ecb_decrypt(&sealed[..20], &key).is_none());
        assert_eq!(zero_pad(b"abc").len(), 16);
        assert_eq!(zero_pad(&[1; 17]).len(), 32);
    }

    #[test]
    fn the_classic_lookup_lays_out_id_address_version_key() {
        let did = p2p_did_bytes(DID).unwrap();
        let own = SocketAddrV4::new(Ipv4Addr::new(192, 0, 2, 1), 12345);
        let key = "XXXXXXXXXXXXXXXXXXXX";
        let payload = lookup_with_key_payload(&did, own, key);
        assert_eq!(payload.len(), 20 + 16 + 4 + key.len() + 4);
        assert_eq!(&payload[..20], &did);
        assert_eq!(&payload[20..22], &[0, 2]);
        assert_eq!(u16::from_le_bytes([payload[22], payload[23]]), 12345);
        assert_eq!(&payload[24..28], &[1, 2, 0, 192]);
        assert_eq!(&payload[28..36], &[0; 8]);
        assert_eq!(&payload[36..40], &[2, 5, 1, 5]);
        assert_eq!(&payload[40..60], key.as_bytes());
        assert_eq!(&payload[60..], &[0; 4]);
        let mut response = vec![0; 4];
        response.extend_from_slice(&payload[20..36]);
        assert_eq!(read_address(&response, 4), Some(own));
        let variant2 = lookup_with_key2_payload(&did, key);
        assert_eq!(&payload[40..], &variant2[20..]);
        assert_eq!(payload.len(), variant2.len() + 20);
    }

    #[test]
    fn relay_payloads_and_answers() {
        let did = p2p_did_bytes(DID).unwrap();
        let relay = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 40000);
        let request = relay_request_payload(&did, relay, 0x0102_0304);
        assert_eq!(&request[..20], &did);
        assert_eq!(read_address(&request, 20), Some(relay));
        assert_eq!(&request[36..], &[1, 2, 3, 4]);
        let packet = relay_packet_payload(9, &did);
        assert_eq!(&packet[..4], &[0, 0, 0, 9]);
        assert_eq!(&packet[4..24], &did);
        assert_eq!(&packet[24..], &[0; 4]);
        assert_eq!(check_cam_payload(&did).len(), 23);

        let mut list = frame_message(response::LIST_REQ_ACK, &[2, 0, 0, 0]);
        list.extend_from_slice(&encode_address(relay));
        list.extend_from_slice(&[0; 16]);
        assert_eq!(parse_relay_list(&list), vec![relay]);
        let mut truncated = list.clone();
        truncated[4] = 5;
        assert_eq!(parse_relay_list(&truncated), vec![relay]);

        let ack = frame_message(response::RLY_PORT_ACK, &[0, 0, 0, 7, 0x9c, 0x40, 0, 0]);
        assert_eq!(parse_relay_port_ack(&ack), Some((7, 40000)));
        assert_eq!(parse_relay_port_ack(&ack[..8]), None);

        let mut to = frame_message(response::RLY_TO, &encode_address(relay));
        to.extend_from_slice(&[0, 0, 1, 0]);
        assert_eq!(parse_relay_to(&to), Some((relay, 256)));
        assert_eq!(parse_relay_to(&to[..20]), None);
    }

    #[test]
    fn packets_and_frame_headers() {
        assert_eq!(frame_message(request::PING, &[]), vec![0xf1, 0xe0, 0, 0]);
        assert_eq!(
            frame_message(request::DATA, &[1, 2, 3])[..4],
            [0xf1, 0xd0, 0, 3]
        );
        assert!(has_header(&[0xf1, 0x42, 0, 0], response::CAM_ID));
        assert_eq!(ack_payload(0, 0x1234), [0xd1, 0, 0, 1, 0x12, 0x34]);
        let ack = frame_message(response::ACK, &[0xd1, 0, 0, 2, 0, 5, 0, 6]);
        assert_eq!(parse_ack(&ack, data_type::DATA), vec![5, 6]);
        assert!(parse_ack(&ack, data_type::VIDEO).is_empty());

        let mut data = command_header(0x0102, 1100).to_vec();
        data.extend(void_body(255));
        assert_eq!(
            data,
            vec![
                0xd1, 0, 1, 2, b'X', b'Z', b'Y', b'H', 0x4c, 0x04, 0, 0, 0, 0, 1, 0, 255, 0, 0, 0
            ]
        );
        let header = parse_frame_header(&data[4..]).unwrap();
        assert_eq!(
            header,
            FrameHeader {
                command: 1100,
                length: 0,
                channel: 255,
                sign_code: 0,
                kind: 0
            }
        );
        let body = command_body(&[9; 36], 3, 8, [0x08, 0]);
        let mut frame = command_header(0, 1350)[4..].to_vec();
        frame.extend(body);
        let header = parse_frame_header(&frame).unwrap();
        assert_eq!(
            (
                header.command,
                header.length,
                header.channel,
                header.sign_code
            ),
            (1350, 36, 3, 8)
        );
        assert_eq!(&frame[10..12], &[0x08, 0]);
        assert!(parse_frame_header(b"XZYH").is_none());
        assert_eq!(nul_terminated(b"{\"a\":1}\0\0junk"), "{\"a\":1}");
    }

    #[test]
    fn level2_bodies_seal_and_open_with_the_counting_sub_header() {
        let key = [7u8; 32];
        let nonce = *b"0123456789ab";
        let plain = br#"{"account_id":"x","cmd":1306,"mChannel":255}"#;
        let sealed = seal_level2(&key, nonce, 0, plain);
        assert_eq!(&sealed[16..28], &nonce);
        assert_eq!(&sealed[28..32], &[0, 3, 2, 1]);
        assert_eq!(open_level2(&key, &sealed, 8).unwrap(), plain);
        assert_eq!(
            &seal_level2(&key, nonce, 255, b"{}")[28..32],
            &[0xff, 3, 2, 1]
        );
        assert_eq!(&seal_level2(&key, nonce, 256, b"{}")[28..32], &[0, 4, 2, 1]);
        // Sign code 2 (device content): no sub-header, ciphertext at 28.
        let mut device = sealed[..28].to_vec();
        device.extend_from_slice(&sealed[32..]);
        assert_eq!(open_level2(&key, &device, 2).unwrap(), plain);
        // A wrong key or a disturbed byte fails closed.
        assert!(open_level2(&[9; 32], &sealed, 8).is_none());
        let mut tampered = sealed.clone();
        tampered[40] ^= 1;
        assert!(open_level2(&key, &tampered, 8).is_none());
        assert!(open_level2(&key, &sealed[..20], 8).is_none());
    }

    #[test]
    fn the_kdf_chains_blocks() {
        let kdf = eufy_kdf(b"shared", 48);
        assert_eq!(kdf.len(), 48);
        assert_eq!(&eufy_kdf(b"shared", 64)[..48], &kdf[..]);
    }

    /// A gateway reply built as the device builds it, for a synthetic
    /// cipher key pair (the SDK's `p2p_level2.spec.ts` does the same).
    fn gateway_reply(
        session_key: &[u8; 32],
        cipher_public: &p256::PublicKey,
        cipher_id: u16,
    ) -> Vec<u8> {
        use cbc::cipher::BlockEncryptMut;
        use p256::elliptic_curve::sec1::ToEncodedPoint;
        let ephemeral = p256::SecretKey::from_slice(&[0x42; 32]).unwrap();
        let shared =
            p256::ecdh::diffie_hellman(ephemeral.to_nonzero_scalar(), cipher_public.as_affine());
        let kdf = eufy_kdf(shared.raw_secret_bytes(), 48);
        let key: [u8; 16] = kdf[..16].try_into().unwrap();
        let iv = [0x11u8; 16];
        let mut plain = session_key.to_vec();
        plain.extend_from_slice(&[0x10; 16]);
        let mut encryptor = cbc::Encryptor::<Aes128>::new(&key.into(), &iv.into());
        for block in plain.chunks_exact_mut(16) {
            encryptor.encrypt_block_mut(block.into());
        }
        let mut out = cipher_id.to_le_bytes().to_vec();
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(ephemeral.public_key().to_encoded_point(true).as_bytes());
        out.extend_from_slice(&iv);
        out.extend(plain);
        out
    }

    #[test]
    fn the_level2_key_unwraps_from_a_gateway_reply_with_the_ecc_key() {
        let cipher = p256::SecretKey::from_slice(&[0x07; 32]).unwrap();
        let reply = gateway_reply(&[0x5a; 32], &cipher.public_key(), 97);
        assert_eq!(gateway_cipher_id(&reply), 97);
        let private_hex = hex::encode(cipher.to_bytes());
        assert_eq!(
            level2_key_from_gateway_ecc(&reply, &private_hex),
            Some([0x5a; 32])
        );
        let other = hex::encode(p256::SecretKey::from_slice(&[0x08; 32]).unwrap().to_bytes());
        assert_ne!(
            level2_key_from_gateway_ecc(&reply, &other),
            Some([0x5a; 32])
        );
        assert_eq!(
            level2_key_from_gateway_ecc(&reply[..40], &private_hex),
            None
        );
        assert_eq!(gateway_cipher_id(&[]), 0);
    }

    /// A synthetic 1024-bit key, generated for this test.
    const TEST_RSA_PKCS8: &str = concat!(
        "MIICeAIBADANBgkqhkiG9w0BAQEFAASCAmIwggJeAgEAAoGBAMEIU5tFNXxXhsYCc98I4M9pMmtY0ZZztOyJFSD4Wptm53Ru",
        "2zAE1IWBr99j0M7xlXSfiUoJkGO/diRcm6YhOcgY/4h0hDjxMSUZbqx2AzmSBd30ucFhSA5GuVf3j2N0cY7aW2k5RIucWEVG",
        "5VMi5A3oTkgu1qOwDHEfTi2Zx8w9AgMBAAECgYEAlTkBoOu8sDXdex8UQhvT5qM1FeUgiMVD3QMP8V/sc2v2i3zTNujU/axG",
        "sFOybPg3CGP4NuLz2xh8lRl+Pcw7pIhc/n9CHHLblThrmNZXVRr8oRwrjMRedkWyP56hv2x1Ud/h6s3PPw5e61D5Go1Tgfsm",
        "s9yy1oBDZMabTlAoAGUCQQD+LJfMHVda+0IN/jINYzKCouafylHCKdrq0KGX/5Xw0lLEh6MTVNTrc1aAj6vzLkd7csNSH5Mg",
        "9m730T9BoKkjAkEAwmtMesfE00QFDSSlgtWKp1jCM1WnRpK5fCeV0o5VA6f5hrJ0xWN1UTdEkKjWgWljK8Gbbo+vh+A5X5dN",
        "k2r7HwJAA45yLUv3zfEYEoQNBTzvFv7ahT/adwII6yIyMKNF4NHAkLlpIbsdI5JAkpnwCP4LyAFqFDsW3u4zzUnjPbLWywJB",
        "AMCPIYqh/bH5tr8TwtJYyiGBaacKeMkPXaM4Wc3XpyAd6Kiea8VtAq9T/BcDAqHeTZjzAw14RrRfQrQI2Y7e8KUCQQDe1pls",
        "CayBJM/vu/nSQBY9AV5+WC/pU5nXZ62nyikeFOcTsGmQ5L7KoIlDcd/PGhgCxSUNEhKvPMdSSpQ33wj2"
    );

    #[test]
    fn the_level2_key_unwraps_from_a_gateway_reply_with_the_rsa_key() {
        use base64::{Engine, engine::general_purpose::STANDARD};
        use rsa::pkcs8::DecodePrivateKey;
        let private =
            rsa::RsaPrivateKey::from_pkcs8_der(&STANDARD.decode(TEST_RSA_PKCS8).unwrap()).unwrap();
        let public = rsa::RsaPublicKey::from(&private);
        let sealed = public
            .encrypt(&mut rand_core::OsRng, rsa::Pkcs1v15Encrypt, &[0x33; 16])
            .unwrap();
        let mut reply = vec![5, 0, 0, 0];
        reply.extend(sealed);
        reply.extend_from_slice(&[0xee; 8]);
        assert_eq!(
            level2_key_from_gateway_rsa(&reply, TEST_RSA_PKCS8),
            Some([0x33; 16])
        );
        // Not a 16-byte key under the padding: refused.
        let wrong = public
            .encrypt(&mut rand_core::OsRng, rsa::Pkcs1v15Encrypt, &[0x33; 24])
            .unwrap();
        let mut reply = vec![5, 0, 0, 0];
        reply.extend(wrong);
        assert_eq!(level2_key_from_gateway_rsa(&reply, TEST_RSA_PKCS8), None);
        // Random bytes in place of the ciphertext: refused, not garbage.
        let mut reply = vec![5, 0, 0, 0];
        reply.extend_from_slice(&[0x01; 128]);
        assert_eq!(level2_key_from_gateway_rsa(&reply, TEST_RSA_PKCS8), None);
        assert_eq!(
            level2_key_from_gateway_rsa(&[5, 0, 0, 0, 1], TEST_RSA_PKCS8),
            None
        );
        assert_eq!(level2_key_from_gateway_rsa(&reply, "not base64"), None);
    }
}
