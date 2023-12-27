use {
    serde::{Deserialize, Serialize},
    std::net::IpAddr,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TPLinkDiscoveryRes {
    pub(crate) system: TPLinkDiscoverySysInfo,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TPLinkDiscoverySysInfo {
    pub(crate) get_sysinfo: GetSysInfo,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum GetSysInfo {
    TPLinkDiscoveryData(TPLinkDiscoveryData),
    TPLinkSmartLightData(TPLinkSmartLightData),
    Empty(()),
    CatchAll(serde_json::Value), // Catch-all variant
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TPLinkDiscoveryData {
    pub active_mode: String,
    pub alias: String,
    pub dev_name: String,
    pub deviceId: String,
    pub err_code: u64,
    pub feature: String,
    pub hwId: String,
    pub hw_ver: String,
    pub icon_hash: String,
    pub latitude_i: i64,
    pub led_off: u64,
    pub longitude_i: i64,
    pub mac: String,
    pub mic_type: String,
    pub model: String,
    pub obd_src: String,
    pub oemId: String,
    pub on_time: i64,
    pub relay_state: u8,
    pub rssi: i64,
    pub status: String,
    pub sw_ver: String,
    pub updating: u64,
    pub ip: Option<IpAddr>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DeviceData {
    SmartPlug(TPLinkDiscoveryData),
    SmartLight(TPLinkSmartLightData),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TPLinkSmartLightData {
    pub alias: String,
    pub light_state: LightState,
    pub is_dimmable: u8,
    pub is_color: u8,
    pub ip: Option<IpAddr>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ControlProtocols {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LightState {
    pub on_off: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DefaultOnState {
    pub mode: String,
    pub hue: u32,
    pub saturation: u32,
    pub color_temp: u32,
    pub brightness: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PreferredState {
    pub index: u8,
    pub hue: u32,
    pub saturation: u32,
    pub color_temp: u32,
    pub brightness: u32,
}
