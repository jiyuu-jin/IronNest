use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Stoplight {
    pub red: bool,
    pub yellow: bool,
    pub green: bool,
}
