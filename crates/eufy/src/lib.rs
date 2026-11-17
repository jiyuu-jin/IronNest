//! A client for the eufy security cloud: sign in (the "mega" `algo_ecdh`
//! API), keep and resume the session, list the account's devices, read a
//! lock's state from the listing, and lock or unlock a smart lock over
//! eufy's P2P protocol ([`Cloud::set_locked`]).
//!
//! The protocol is as documented, and verified against live traffic, by
//! mega-yfue/eufy-sdk (Apache-2.0); [`lock`] says which parts of the lock
//! path that SDK has verified on which hardware.

pub mod cloud;
pub mod crypto;
pub mod ff09;
pub mod lock;
pub mod p2p;

pub use cloud::{Cloud, Config, Device, Login, Session};
pub use lock::{Actuation, Wire};
