use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RokuDiscoverRes {
    pub location: String,
    pub usn: String,
    pub server: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ActionApp {
    pub app: Vec<App>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct App {
    pub id: String,
    #[serde(rename = "type")]
    pub app_type: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RokuDeviceInfo {
    #[serde(rename = "user-device-name")]
    pub user_device_name: String,
    #[serde(rename = "power-mode")]
    pub power_mode: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Apps {
    #[serde(rename = "app", default)]
    pub apps: Vec<AppsApp>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppsApp {
    #[serde(rename = "$value")]
    pub name: String,

    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "type")]
    pub app_type: String,

    #[serde(rename = "version")]
    pub version: String,
}
