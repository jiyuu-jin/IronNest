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
    TPLinkDiscoveryData(Box<TPLinkDiscoveryData>),
    TPLinkSmartLightData(TPLinkSmartLightData),
    TPLinkSmartPowerStripData(Box<TPLinkSmartPowerStripRes>),
    Empty(()),
    CatchAll(serde_json::Value), // Catch-all variant
}

// @TPLink Smart Power Strip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPLinkSmartPowerStripRes {
    pub alias: String,
    pub child_num: i32,
    pub children: Vec<Child>,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub err_code: i32,
    pub feature: String,
    #[serde(rename = "hwId")]
    pub hw_id: String,
    pub hw_ver: String,
    pub latitude_i: i32,
    pub led_off: i32,
    pub longitude_i: i32,
    pub mac: String,
    pub mic_type: String,
    pub model: String,
    pub ntc_state: Option<i32>,
    pub obd_src: Option<String>,
    #[serde(rename = "oemId")]
    pub oem_id: String,
    pub rssi: i32,
    pub status: String,
    pub sw_ver: String,
    pub updating: i32,
    pub ip: Option<IpAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Child {
    pub alias: String,
    pub id: String,
    pub next_action: NextAction,
    pub on_time: i32,
    pub state: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    #[serde(rename = "type")]
    pub action_type: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TPLinkDiscoveryData {
    pub active_mode: String,
    pub alias: String,
    pub dev_name: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub err_code: u64,
    pub feature: String,
    #[serde(rename = "hwId")]
    pub hw_id: String,
    pub hw_ver: String,
    pub icon_hash: String,
    pub latitude_i: i64,
    pub led_off: u64,
    pub longitude_i: i64,
    pub mac: String,
    pub mic_type: String,
    pub model: String,
    pub obd_src: String,
    #[serde(rename = "oemId")]
    pub oem_id: String,
    pub on_time: i64,
    pub relay_state: i32,
    pub rssi: i64,
    pub status: String,
    pub sw_ver: String,
    pub updating: u64,
    pub ip: Option<IpAddr>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DeviceData {
    SmartPlug(Box<TPLinkDiscoveryData>),
    SmartLight(TPLinkSmartLightData),
    SmartDimmer(Box<TPLinkDiscoveryData>),
    SmartPowerStrip(Box<TPLinkSmartPowerStripRes>),
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
    pub on_off: i32,
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
