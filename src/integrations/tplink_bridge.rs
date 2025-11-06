use {
    crate::integrations::iron_nest::types::{Device as IronNestDevice, DeviceType},
    chrono::Utc,
    std::sync::Arc,
    tplink::{Device as TpLinkDevice, SafeDeviceRegistry},
};

/// Bridge between TP-Link devices and Iron Nest database records
pub struct TpLinkBridge {
    registry: SafeDeviceRegistry,
}

impl TpLinkBridge {
    /// Create a new TP-Link bridge
    pub fn new() -> Self {
        Self {
            registry: tplink::create_registry(),
        }
    }

    /// Get the device registry for direct access
    pub fn registry(&self) -> &SafeDeviceRegistry {
        &self.registry
    }

    /// Refresh devices from network and return Iron Nest compatible records
    pub async fn refresh_devices(&self) -> Result<Vec<IronNestDevice>, tplink::TpLinkError> {
        let mut registry = self.registry.write().await;
        let tplink_devices = registry.discover_and_register().await?;

        let iron_nest_devices = tplink_devices
            .into_iter()
            .filter_map(|device| self.to_iron_nest_device(&device))
            .collect();

        Ok(iron_nest_devices)
    }

    /// Get a TP-Link device by IP address
    pub async fn get_device(&self, ip: &str) -> Option<TpLinkDevice> {
        let registry = self.registry.read().await;
        registry.get_device(ip).cloned()
    }

    /// Get a smart plug by IP address
    pub async fn get_smart_plug(&self, ip: &str) -> Option<tplink::SmartPlug> {
        let registry = self.registry.read().await;
        registry.get_smart_plug(ip).cloned()
    }

    /// Get a smart light by IP address
    pub async fn get_smart_light(&self, ip: &str) -> Option<tplink::SmartLight> {
        let registry = self.registry.read().await;
        registry.get_smart_light(ip).cloned()
    }

    /// Get a smart dimmer by IP address
    pub async fn get_smart_dimmer(&self, ip: &str) -> Option<tplink::SmartDimmer> {
        let registry = self.registry.read().await;
        registry.get_smart_dimmer(ip).cloned()
    }

    /// Get a power strip by IP address
    pub async fn get_power_strip(&self, ip: &str) -> Option<tplink::PowerStrip> {
        let registry = self.registry.read().await;
        registry.get_power_strip(ip).cloned()
    }

    /// Convert TP-Link device to Iron Nest database record
    fn to_iron_nest_device(&self, device: &TpLinkDevice) -> Option<IronNestDevice> {
        let ip = device.ip_string()?;
        let alias = device.alias();
        let mac_address = device.mac_address().map(|s| s.to_string());

        let (device_type, child_id) = match device {
            TpLinkDevice::SmartPlug(_) => (DeviceType::KasaPlug, None),
            TpLinkDevice::SmartLight(_) => (DeviceType::KasaLight, None),
            TpLinkDevice::SmartDimmer(_) => (DeviceType::KasaDimmer, None),
            TpLinkDevice::PowerStrip(_strip) => {
                // For power strips, we'll create a record for the main device
                // Individual sockets can be handled separately if needed
                (DeviceType::KasaPowerStrip, None)
            }
        };

        Some(IronNestDevice {
            id: 0, // Will be set by database
            name: alias.to_string(),
            device_type,
            ip,
            power_state: 0,          // Will be updated separately
            battery_percentage: 100, // TP-Link devices don't have batteries
            last_seen: Utc::now(),
            mac_address,
            child_id,
        })
    }
}

impl Default for TpLinkBridge {
    fn default() -> Self {
        Self::new()
    }
}

// Global instance
use std::sync::OnceLock;
static TPLINK_BRIDGE: OnceLock<Arc<TpLinkBridge>> = OnceLock::new();

/// Get the global TP-Link bridge instance
pub fn get_tplink_bridge() -> &'static Arc<TpLinkBridge> {
    TPLINK_BRIDGE.get_or_init(|| Arc::new(TpLinkBridge::new()))
}
