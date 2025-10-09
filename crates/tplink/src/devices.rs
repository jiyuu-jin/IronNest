use {
    crate::{
        client::TpLinkClient,
        error::Result,
        types::{TPLinkDiscoveryData, TPLinkSmartLightData, TPLinkSmartPowerStripRes},
    },
    serde_json::Value,
    std::{net::IpAddr, sync::Arc},
};

/// A TP-Link device with embedded client for operations
#[derive(Clone, Debug)]
pub enum Device {
    SmartPlug(SmartPlug),
    SmartLight(SmartLight),
    SmartDimmer(SmartDimmer),
    PowerStrip(PowerStrip),
}

impl Device {
    /// Get the device's IP address
    pub fn ip(&self) -> Option<IpAddr> {
        match self {
            Device::SmartPlug(d) => d.data.ip,
            Device::SmartLight(d) => d.data.ip,
            Device::SmartDimmer(d) => d.data.ip,
            Device::PowerStrip(d) => d.data.ip,
        }
    }

    /// Get the device's IP address as a string
    pub fn ip_string(&self) -> Option<String> {
        self.ip().map(|ip| ip.to_string())
    }

    /// Get the device's alias/name
    pub fn alias(&self) -> &str {
        match self {
            Device::SmartPlug(d) => &d.data.alias,
            Device::SmartLight(d) => &d.data.alias,
            Device::SmartDimmer(d) => &d.data.alias,
            Device::PowerStrip(d) => &d.data.alias,
        }
    }

    /// Get the device's unique identifier
    pub fn device_id(&self) -> &str {
        match self {
            Device::SmartPlug(d) => &d.data.device_id,
            Device::SmartLight(_) => "light", // Light data doesn't have device_id in current structure
            Device::SmartDimmer(d) => &d.data.device_id,
            Device::PowerStrip(d) => &d.data.device_id,
        }
    }

    /// Get the device's MAC address
    pub fn mac_address(&self) -> Option<&str> {
        match self {
            Device::SmartPlug(d) => Some(&d.data.mac),
            Device::SmartLight(_) => None, // Light data doesn't have MAC in current structure
            Device::SmartDimmer(d) => Some(&d.data.mac),
            Device::PowerStrip(d) => Some(&d.data.mac),
        }
    }
}

/// Smart plug device with control operations
#[derive(Clone, Debug)]
pub struct SmartPlug {
    pub data: TPLinkDiscoveryData,
    client: Arc<TpLinkClient>,
}

impl SmartPlug {
    pub fn new(data: TPLinkDiscoveryData, client: Arc<TpLinkClient>) -> Self {
        Self { data, client }
    }

    /// Turn the smart plug on
    pub async fn turn_on(&self) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.turn_plug_on(&ip).await
    }

    /// Turn the smart plug off
    pub async fn turn_off(&self) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.turn_plug_off(&ip).await
    }

    /// Set the device alias/name
    pub async fn set_alias(&self, alias: &str) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.set_plug_alias(&ip, alias).await
    }

    /// Reboot the device
    pub async fn reboot(&self) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.reboot_plug(&ip).await
    }

    pub fn ip_string(&self) -> Option<String> {
        self.data.ip.map(|ip| ip.to_string())
    }

    pub fn alias(&self) -> &str {
        &self.data.alias
    }

    pub fn device_id(&self) -> &str {
        &self.data.device_id
    }
}

/// Smart light device with control operations
#[derive(Clone, Debug)]
pub struct SmartLight {
    pub data: TPLinkSmartLightData,
    client: Arc<TpLinkClient>,
}

impl SmartLight {
    pub fn new(data: TPLinkSmartLightData, client: Arc<TpLinkClient>) -> Self {
        Self { data, client }
    }

    /// Turn the light on or off
    pub async fn turn_on_off(&self, on: bool) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.turn_light_on_off(&ip, on).await
    }

    /// Turn the light on
    pub async fn turn_on(&self) -> Result<()> {
        self.turn_on_off(true).await
    }

    /// Turn the light off
    pub async fn turn_off(&self) -> Result<()> {
        self.turn_on_off(false).await
    }

    /// Set brightness (0-100)
    pub async fn set_brightness(&self, brightness: u8) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.set_light_brightness(&ip, brightness).await
    }

    /// Set color using CSS color format
    pub async fn set_color(&self, color: &str) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.set_light_color(&ip, color).await
    }

    pub fn ip_string(&self) -> Option<String> {
        self.data.ip.map(|ip| ip.to_string())
    }

    pub fn alias(&self) -> &str {
        &self.data.alias
    }

    pub fn is_dimmable(&self) -> bool {
        self.data.is_dimmable == 1
    }

    pub fn is_color(&self) -> bool {
        self.data.is_color == 1
    }
}

/// Smart dimmer device with control operations
#[derive(Clone, Debug)]
pub struct SmartDimmer {
    pub data: TPLinkDiscoveryData,
    client: Arc<TpLinkClient>,
}

impl SmartDimmer {
    pub fn new(data: TPLinkDiscoveryData, client: Arc<TpLinkClient>) -> Self {
        Self { data, client }
    }

    /// Set brightness (0-100)
    pub async fn set_brightness(&self, brightness: u8) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.set_dimmer_brightness(&ip, brightness).await
    }

    /// Set inactivity timeout in minutes
    pub async fn set_inactivity_timeout(&self, timeout_minutes: u8) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client
            .set_dimmer_inactivity_timeout(&ip, timeout_minutes)
            .await
    }

    pub fn ip_string(&self) -> Option<String> {
        self.data.ip.map(|ip| ip.to_string())
    }

    pub fn alias(&self) -> &str {
        &self.data.alias
    }

    pub fn device_id(&self) -> &str {
        &self.data.device_id
    }
}

/// Power strip device with socket control operations
#[derive(Clone, Debug)]
pub struct PowerStrip {
    pub data: TPLinkSmartPowerStripRes,
    client: Arc<TpLinkClient>,
}

impl PowerStrip {
    pub fn new(data: TPLinkSmartPowerStripRes, client: Arc<TpLinkClient>) -> Self {
        Self { data, client }
    }

    /// Turn on a specific socket
    pub async fn turn_socket_on(&self, socket_id: &str) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client.turn_power_strip_socket_on(&ip, socket_id).await
    }

    /// Turn off a specific socket
    pub async fn turn_socket_off(&self, socket_id: &str) -> Result<()> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client
            .turn_power_strip_socket_off(&ip, socket_id)
            .await
    }

    /// Get energy usage for a specific socket
    pub async fn get_energy_usage(&self, socket_id: &str) -> Result<Value> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client
            .get_power_strip_energy_usage(&ip, socket_id)
            .await
    }

    /// Get emeter data for a specific socket
    pub async fn get_emeter_data(&self, socket_id: &str) -> Result<Value> {
        let ip =
            self.ip_string()
                .ok_or_else(|| crate::error::TpLinkError::DeviceCommunication {
                    message: "Device has no IP address".to_string(),
                })?;
        self.client
            .get_power_strip_emeter_data(&ip, socket_id)
            .await
    }

    pub fn ip_string(&self) -> Option<String> {
        self.data.ip.map(|ip| ip.to_string())
    }

    pub fn alias(&self) -> &str {
        &self.data.alias
    }

    pub fn device_id(&self) -> &str {
        &self.data.device_id
    }

    pub fn sockets(&self) -> &[crate::types::Child] {
        &self.data.children
    }
}
