use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub ip: String,
    pub state: String,
}
