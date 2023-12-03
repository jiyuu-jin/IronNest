use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RokuDiscoverRes {
    pub location: String,
    pub usn: String,
    pub server: String,
}
