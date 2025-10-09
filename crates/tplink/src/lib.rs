//! # TP-Link Smart Device Client Library
//!
//! A pure Rust library for communicating with TP-Link smart devices including
//! smart plugs, smart lights, smart dimmers, and power strips.
//!
//! ## Features
//!
//! - Device discovery via UDP broadcast
//! - Control smart plugs (on/off, alias, reboot)
//! - Control smart lights (on/off, brightness, color)
//! - Control smart dimmers (brightness, inactivity timeout)
//! - Control power strips (individual socket control, energy monitoring)
//! - Async/await support with Tokio
//! - Proper error handling
//!
//! ## Examples
//!
//! ### Discover devices on the network
//!
//! ```rust,no_run
//! use tplink::{discover_devices, DeviceData};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let devices = discover_devices().await?;
//!     
//!     for device in devices {
//!         match device {
//!             DeviceData::SmartPlug(plug) => {
//!                 println!("Found smart plug: {}", plug.alias);
//!             }
//!             DeviceData::SmartLight(light) => {
//!                 println!("Found smart light: {}", light.alias);
//!             }
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Control a smart plug
//!
//! ```rust,no_run
//! use tplink::TpLinkClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = TpLinkClient::new();
//!     
//!     // Turn on the plug
//!     client.turn_plug_on("192.168.1.100").await?;
//!     
//!     // Set brightness if it's a dimmer
//!     client.set_dimmer_brightness("192.168.1.101", 75).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod devices;
pub mod discovery;
pub mod error;
pub mod protocol;
pub mod registry;
pub mod types;

// Re-export the most commonly used items
pub use {
    client::TpLinkClient,
    devices::{Device, PowerStrip, SmartDimmer, SmartLight, SmartPlug},
    discovery::{discover, discover_devices, discover_devices_with_client},
    error::{Result, TpLinkError},
    registry::{create_registry, DeviceRegistry, SafeDeviceRegistry},
    types::*,
};

// Legacy standalone functions for backward compatibility

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = TpLinkClient::new();
    }
}
