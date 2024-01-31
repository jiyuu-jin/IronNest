use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TuyaDeviceRes {
    pub result: Vec<TuyaDeviceResResult>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TuyaDeviceResResult {
    pub ip: String,
    pub local_key: String,
    pub uid: String,
    pub name: String,
    pub product_name: String,
}
