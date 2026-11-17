//! eufy's P2P protocol (ThroughTek PPCS), as far as commanding a lock
//! needs: reaching the device (lookup, hole punch or relay), negotiating
//! the level-2 session key, and sending sealed `SET_PAYLOAD` commands.
//!
//! Ported from mega-yfue/eufy-sdk (Apache-2.0), `src/transport/p2p/`.

pub mod codec;
pub mod session;

pub use session::{CipherKeys, Config, Event, Session};
