use {
    crate::{client::TpLinkClient, devices::Device, error::Result},
    std::{collections::HashMap, sync::Arc},
    tokio::sync::RwLock,
};

/// Registry for managing discovered TP-Link devices
#[derive(Debug)]
pub struct DeviceRegistry {
    devices: HashMap<String, Device>, // keyed by IP address
    client: Arc<TpLinkClient>,
}

impl DeviceRegistry {
    /// Create a new device registry with a shared client
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            client: Arc::new(TpLinkClient::new()),
        }
    }

    /// Create a new device registry with a specific client
    pub fn with_client(client: Arc<TpLinkClient>) -> Self {
        Self {
            devices: HashMap::new(),
            client,
        }
    }

    /// Discover devices on the network and register them
    pub async fn discover_and_register(&mut self) -> Result<Vec<Device>> {
        let devices = crate::discovery::discover_devices_with_client(self.client.clone()).await?;

        // Clear existing devices and register new ones
        self.devices.clear();
        for device in &devices {
            if let Some(ip) = device.ip_string() {
                self.devices.insert(ip, device.clone());
            }
        }

        Ok(devices)
    }

    /// Get a device by IP address
    pub fn get_device(&self, ip: &str) -> Option<&Device> {
        self.devices.get(ip)
    }

    /// Get a smart plug by IP address
    pub fn get_smart_plug(&self, ip: &str) -> Option<&crate::devices::SmartPlug> {
        match self.devices.get(ip) {
            Some(Device::SmartPlug(plug)) => Some(plug),
            _ => None,
        }
    }

    /// Get a smart light by IP address
    pub fn get_smart_light(&self, ip: &str) -> Option<&crate::devices::SmartLight> {
        match self.devices.get(ip) {
            Some(Device::SmartLight(light)) => Some(light),
            _ => None,
        }
    }

    /// Get a smart dimmer by IP address
    pub fn get_smart_dimmer(&self, ip: &str) -> Option<&crate::devices::SmartDimmer> {
        match self.devices.get(ip) {
            Some(Device::SmartDimmer(dimmer)) => Some(dimmer),
            _ => None,
        }
    }

    /// Get a power strip by IP address
    pub fn get_power_strip(&self, ip: &str) -> Option<&crate::devices::PowerStrip> {
        match self.devices.get(ip) {
            Some(Device::PowerStrip(strip)) => Some(strip),
            _ => None,
        }
    }

    /// Register a single device
    pub fn register_device(&mut self, device: Device) {
        if let Some(ip) = device.ip_string() {
            self.devices.insert(ip, device);
        }
    }

    /// Remove a device by IP
    pub fn remove_device(&mut self, ip: &str) -> Option<Device> {
        self.devices.remove(ip)
    }

    /// Get all registered devices
    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }

    /// Get the number of registered devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Clear all registered devices
    pub fn clear(&mut self) {
        self.devices.clear();
    }

    /// Get the shared client
    pub fn client(&self) -> &Arc<TpLinkClient> {
        &self.client
    }
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe device registry
pub type SafeDeviceRegistry = Arc<RwLock<DeviceRegistry>>;

/// Create a new thread-safe device registry
pub fn create_registry() -> SafeDeviceRegistry {
    Arc::new(RwLock::new(DeviceRegistry::new()))
}
