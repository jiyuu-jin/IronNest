//! The eufy cloud's `algo_ecdh` request scheme.
//!
//! - **Key exchange**: an ephemeral P-256 key pair; its uncompressed public
//!   key, as hex text, is sent `base64(IV || AES-128-CBC(text, local key))`
//!   and signed with HMAC-SHA256 keyed by the local key's hex text over
//!   `ts+once+body`. The server answers its public key wrapped the same way;
//!   the session's **share key** is the first 32 hex digits of the ECDH
//!   secret.
//! - **Bodies**: `base64(IV || AES-128-CBC-PKCS7(json))`, keyed by the first
//!   16 bytes of the share key; an answer's `data` decrypts the same way.
//! - **Signature**: HMAC-SHA256 keyed by the share key's first 32 hex digits
//!   as text, over `ts+once[+encrypted body]`, hex.
//! - **Login password**: a one-shot ECDH against the server's fixed public
//!   key; AES-256-CBC keyed by the whole secret, IV its first 16 bytes.
//!
//! As documented and verified against live traffic by mega-yfue/eufy-sdk
//! (Apache-2.0).

use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use base64::{Engine, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use p256::{PublicKey, ecdh::EphemeralSecret, elliptic_curve::sec1::ToEncodedPoint};
use rand_core::{OsRng, RngCore};
use sha2::Sha256;

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

/// The eufy app's bootstrap key for the key exchange on the `*.eufy.com`
/// API hosts.
pub const MEGA_LOCAL_KEY_HEX: &str = "2500a7d5617812f9d52515b2c8f20a3d";

/// The server's fixed P-256 public key the login password is encrypted to.
pub const SERVER_STATIC_PUBLIC_KEY_HEX: &str = "04c5c00c4f8d1197cc7c3167c52bf7acb054d722f0ef08dcd7e0883236e0d72a3868d9750cb47fa4619248f3d83f0f662671dadc6e2d31c2f41db0161651c7c076";

/// 32 random hex digits (an `x-key-ident` or `x-request-once`).
pub fn gen_id() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// The md5 of `text`, as hex.
pub fn md5_hex(text: &str) -> String {
    hex::encode(Md5::digest(text.as_bytes()))
}

/// The `gtoken` header: md5 of the account's user id.
pub fn gtoken(user_id: &str) -> String {
    md5_hex(user_id)
}

fn hmac_hex(key: &[u8], message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("any key length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn random_iv() -> [u8; 16] {
    let mut iv = [0u8; 16];
    OsRng.fill_bytes(&mut iv);
    iv
}

fn wrap_128(key: &[u8; 16], iv: [u8; 16], plaintext: &[u8]) -> String {
    let ciphertext =
        Aes128CbcEnc::new(key.into(), &iv.into()).encrypt_padded_vec_mut::<Pkcs7>(plaintext);
    let mut out = iv.to_vec();
    out.extend(ciphertext);
    STANDARD.encode(out)
}

fn unwrap_128(key: &[u8; 16], b64: &str) -> Result<Vec<u8>, String> {
    let raw = STANDARD
        .decode(b64.trim())
        .map_err(|_| "an encrypted body that is not base64".to_string())?;
    if raw.len() < 32 {
        return Err("an encrypted body too short to hold anything".to_string());
    }
    let (iv, ciphertext) = raw.split_at(16);
    let iv: [u8; 16] = iv.try_into().expect("16 bytes");
    Aes128CbcDec::new(key.into(), &iv.into())
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|_| "an encrypted body that does not decrypt under this key".to_string())
}

/// The session's AES key: the first 16 bytes of the share key.
pub fn aes_key(share_key_hex: &str) -> Result<[u8; 16], String> {
    let bytes = hex::decode(
        share_key_hex
            .get(..32)
            .ok_or("a share key under 32 hex digits")?,
    )
    .map_err(|_| "a share key that is not hex".to_string())?;
    Ok(bytes.try_into().expect("16 bytes"))
}

/// Encrypt a request body under the session's share key.
pub fn encrypt_body(plaintext: &[u8], share_key_hex: &str) -> Result<String, String> {
    Ok(wrap_128(&aes_key(share_key_hex)?, random_iv(), plaintext))
}

/// Decrypt an answer's `data` under the session's share key.
pub fn decrypt_body(b64: &str, share_key_hex: &str) -> Result<Vec<u8>, String> {
    unwrap_128(&aes_key(share_key_hex)?, b64)
}

/// `x-signature` for a request: over `ts+once`, and the encrypted body when
/// there is one.
pub fn sign_request(
    share_key_hex: &str,
    ts: &str,
    once: &str,
    encrypted_body: Option<&str>,
) -> String {
    let key = share_key_hex.get(..32).unwrap_or(share_key_hex);
    let message = match encrypted_body {
        Some(body) if !body.is_empty() => format!("{ts}+{once}+{body}"),
        _ => format!("{ts}+{once}"),
    };
    hmac_hex(key.as_bytes(), &message)
}

/// A key exchange in progress: the ephemeral secret and what to send.
pub struct KeyExchange {
    secret: EphemeralSecret,
    local_key: [u8; 16],
    /// The `x-key-ident` this client minted for the session.
    pub key_ident: String,
    /// The body's `client_public_key`.
    pub encrypted_client_public_key: String,
    /// `x-request-ts`, `x-request-once` and `x-signature` for the exchange.
    pub ts: String,
    pub once: String,
    pub signature: String,
}

/// A negotiated session key.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionKey {
    pub key_ident: String,
    /// 32 hex digits.
    pub share_key: String,
}

/// Start a key exchange under the bootstrap `local_key_hex`, at `ts`.
pub fn prepare_key_exchange(local_key_hex: &str, ts: &str) -> Result<KeyExchange, String> {
    let local_key: [u8; 16] = hex::decode(local_key_hex)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or("a bootstrap key that is not 16 bytes of hex")?;
    let secret = EphemeralSecret::random(&mut OsRng);
    let public_hex = hex::encode(secret.public_key().to_encoded_point(false).as_bytes());
    let encrypted = wrap_128(&local_key, random_iv(), public_hex.as_bytes());
    let once = gen_id();
    let signature = hmac_hex(
        local_key_hex.as_bytes(),
        &format!("{ts}+{once}+{encrypted}"),
    );
    Ok(KeyExchange {
        secret,
        local_key,
        key_ident: gen_id(),
        encrypted_client_public_key: encrypted,
        ts: ts.to_string(),
        once,
        signature,
    })
}

/// The ECDH secret with a peer's uncompressed public key given as hex.
fn shared_secret(secret: &EphemeralSecret, peer_hex: &str) -> Result<[u8; 32], String> {
    let peer =
        hex::decode(peer_hex.trim()).map_err(|_| "a public key that is not hex".to_string())?;
    let peer =
        PublicKey::from_sec1_bytes(&peer).map_err(|_| "a public key not on P-256".to_string())?;
    let shared = secret.diffie_hellman(&peer);
    Ok((*shared.raw_secret_bytes()).into())
}

impl KeyExchange {
    /// Finish with the server's wrapped public key.
    pub fn finish(self, server_public_key_b64: &str) -> Result<SessionKey, String> {
        let server_hex = unwrap_128(&self.local_key, server_public_key_b64)?;
        let server_hex = String::from_utf8(server_hex)
            .map_err(|_| "a server key that is not text".to_string())?;
        let shared = shared_secret(&self.secret, &server_hex)?;
        Ok(SessionKey {
            key_ident: self.key_ident,
            share_key: hex::encode(shared)[..32].to_string(),
        })
    }
}

/// The login password encrypted to the server's fixed key: the client's
/// public key (hex) and the ciphertext (base64).
pub fn encrypt_login_password(password: &str) -> Result<(String, String), String> {
    let secret = EphemeralSecret::random(&mut OsRng);
    let public_hex = hex::encode(secret.public_key().to_encoded_point(false).as_bytes());
    let shared = shared_secret(&secret, SERVER_STATIC_PUBLIC_KEY_HEX)?;
    let iv: [u8; 16] = shared[..16].try_into().expect("16 bytes");
    let ciphertext = Aes256CbcEnc::new((&shared).into(), &iv.into())
        .encrypt_padded_vec_mut::<Pkcs7>(password.as_bytes());
    Ok((public_hex, STANDARD.encode(ciphertext)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector() -> serde_json::Value {
        serde_json::from_str(include_str!("../tests/vector.json")).expect("the vector")
    }

    #[test]
    fn a_body_decrypts_and_is_signed_as_the_captured_scheme_says() {
        let v = vector();
        let key = v["sharedKey"].as_str().expect("key");
        let plain = decrypt_body(v["body_b64"].as_str().expect("body"), key).expect("decrypts");
        assert_eq!(String::from_utf8(plain).expect("text"), v["body_pt"]);
        let signature = sign_request(
            key,
            v["ts"].as_str().expect("ts"),
            v["once"].as_str().expect("once"),
            v["body_b64"].as_str(),
        );
        assert_eq!(signature, v["sig"]);
        let bare = sign_request(
            key,
            v["ts"].as_str().expect("ts"),
            v["once"].as_str().expect("once"),
            None,
        );
        assert_ne!(bare, signature);
        assert_eq!(bare.len(), 64);
    }

    #[test]
    fn a_body_round_trips_under_a_fresh_iv_each_time() {
        let key = "0123456789abcdef0123456789abcdef";
        let one = encrypt_body(b"{\"a\":1}", key).expect("encrypted");
        let two = encrypt_body(b"{\"a\":1}", key).expect("encrypted");
        assert_ne!(one, two);
        assert_eq!(decrypt_body(&one, key).expect("decrypts"), b"{\"a\":1}");
        assert!(decrypt_body(&one, "fedcba9876543210fedcba9876543210").is_err());
        assert!(decrypt_body("not base64!", key).is_err());
    }

    #[test]
    fn gtoken_is_the_md5_of_the_user_id() {
        assert_eq!(gtoken("abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn a_key_exchange_agrees_with_a_server_doing_the_same_ecdh() {
        let exchange = prepare_key_exchange(MEGA_LOCAL_KEY_HEX, "1700000000").expect("prepared");
        // The client's public key, as the server would unwrap it.
        let local: [u8; 16] = hex::decode(MEGA_LOCAL_KEY_HEX)
            .expect("hex")
            .try_into()
            .expect("16");
        let client_hex = String::from_utf8(
            unwrap_128(&local, &exchange.encrypted_client_public_key).expect("unwraps"),
        )
        .expect("text");
        assert_eq!(client_hex.len(), 130, "uncompressed P-256, as hex");
        // The signature is over ts+once+body with the bootstrap key's hex text.
        assert_eq!(
            exchange.signature,
            hmac_hex(
                MEGA_LOCAL_KEY_HEX.as_bytes(),
                &format!(
                    "1700000000+{}+{}",
                    exchange.once, exchange.encrypted_client_public_key
                )
            )
        );
        // A server key pair; it wraps its public key under the bootstrap key.
        let server = EphemeralSecret::random(&mut OsRng);
        let server_hex = hex::encode(server.public_key().to_encoded_point(false).as_bytes());
        let wrapped = wrap_128(&local, random_iv(), server_hex.as_bytes());
        let server_shared = shared_secret(&server, &client_hex).expect("server side");
        let session = exchange.finish(&wrapped).expect("finished");
        assert_eq!(session.share_key, hex::encode(server_shared)[..32]);
    }

    #[test]
    fn the_login_password_is_aes_256_under_the_whole_secret() {
        let (public_hex, ciphertext) = encrypt_login_password("correct horse").expect("encrypted");
        assert_eq!(public_hex.len(), 130);
        // 13 bytes pad to one 16-byte block.
        assert_eq!(STANDARD.decode(&ciphertext).expect("b64").len(), 16);
        let (_, again) = encrypt_login_password("correct horse").expect("encrypted");
        assert_ne!(ciphertext, again, "a fresh key pair each time");
    }
}
