use {
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::fmt,
    uuid::Uuid,
};

pub mod config;

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
    SmartDimmer,
    SmartPowerStrip,
    RingDoorbell,
    RokuTv,
    Stoplight,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SmartPlug => write!(f, "Smart Plug"),
            Self::SmartLight => write!(f, "Smart Light"),
            Self::SmartDimmer => write!(f, "Smart Dimmer"),
            Self::SmartPowerStrip => write!(f, "Smart Power Strip"),
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
    pub power_state: i32,
    pub battery_percentage: i64,
    pub last_seen: DateTime<Utc>,
    pub mac_address: Option<String>,
    pub child_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct AuthState {
    pub refresh_token: String,
    pub hardware_id: String,
    pub auth_token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct FullAction {
    pub id: Uuid,
    #[serde(flatten)]
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub fields: RequiredAction,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct RequiredAction {
    pub name: String,
    pub cron: String,
    pub function_name: String,
    pub function_args: Value,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Integration {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub image: String,
}

#[derive(Debug)]
pub enum ControlMessage {
    Start,
    Stop,
    Shutdown,
}
