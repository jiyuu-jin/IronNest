use serde::{Deserialize, Serialize};

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
    Empty(()),
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
    pub relay_state: u64,
    pub rssi: i64,
    pub status: String,
    pub sw_ver: String,
    pub updating: u64,
}
