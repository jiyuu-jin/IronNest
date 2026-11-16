//! A client for the eufy security cloud: sign in (the "mega" `algo_ecdh`
//! API), keep and resume the session, list the account's devices and read
//! a lock's state from the listing.
//!
//! Commanding a lock is not here yet: depending on the model it goes over
//! eufy's P2P protocol or its secure MQTT broker, and neither is ported.
//!
//! The protocol is as documented, and verified against live traffic, by
//! mega-yfue/eufy-sdk (Apache-2.0).

pub mod cloud;
pub mod crypto;

pub use cloud::{Cloud, Config, Device, Login, Session};
