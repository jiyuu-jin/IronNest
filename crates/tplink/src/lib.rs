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
pub mod types;

// Re-export the most commonly used items
// Re-export device operations for convenience
pub use devices::{
    get_power_strip_emeter_data,
    get_power_strip_energy_usage,
    parse_power_reading,
    parse_strip_info,
    reboot_plug,
    // Smart dimmers
    set_dimmer_brightness,
    set_dimmer_inactivity_timeout,
    set_light_brightness,
    set_light_color,
    set_plug_alias,
    // Smart lights
    turn_light_on_off,
    turn_plug_off,
    // Smart plugs
    turn_plug_on,
    turn_power_strip_socket_off,
    // Power strips
    turn_power_strip_socket_on,
    Outlet,
    PowerReading,
    StripInfo,
};
pub use {
    client::TpLinkClient,
    discovery::{discover_devices, hosts_in, scan, Found},
    error::{Result, TpLinkError},
    types::*,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let _client = TpLinkClient::new();
    }
}
