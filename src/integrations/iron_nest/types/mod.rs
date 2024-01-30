use {
    serde::{Deserialize, Serialize},
    std::fmt,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::prelude::Type))]
#[serde(rename_all = "kebab-case")]
pub enum DeviceType {
    SmartPlug,
    SmartLight,
    RingDoorbell,
    RokuTv,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SmartPlug => write!(f, "Smart Plug"),
            Self::SmartLight => write!(f, "Smart Light"),
            Self::RingDoorbell => write!(f, "Ring Doorbell"),
            Self::RokuTv => write!(f, "Roku TV"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub device_type: DeviceType,
    pub ip: String,
    pub power_state: u8,
    pub battery_percentage: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AuthState {
    pub refresh_token: String,
    pub hardware_id: String,
    pub auth_token: String,
}
