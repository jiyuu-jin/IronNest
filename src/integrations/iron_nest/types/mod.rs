use {
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    std::fmt,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::prelude::Type))]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(
    feature = "ssr",
    sqlx(type_name = "device_type", rename_all = "kebab-case")
)]
pub enum DeviceType {
    SmartPlug,
    SmartLight,
    RingDoorbell,
    RokuTv,
    Stoplight,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SmartPlug => write!(f, "Smart Plug"),
            Self::SmartLight => write!(f, "Smart Light"),
            Self::RingDoorbell => write!(f, "Ring Doorbell"),
            Self::RokuTv => write!(f, "Roku TV"),
            Self::Stoplight => write!(f, "Stoplight"),
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
    pub power_state: i64,
    pub battery_percentage: i64,
    pub last_seen: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AuthState {
    pub refresh_token: String,
    pub hardware_id: String,
    pub auth_token: String,
}
