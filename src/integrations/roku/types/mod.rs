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
