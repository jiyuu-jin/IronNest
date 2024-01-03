use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CountryDomainResponse {
    pub code: i32,
    pub msg: String,
    pub data: Data,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub domain: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse {
    pub code: i32,
    pub msg: String,
    pub data: ResponseData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseData {
    pub user_id: String,
    pub email: String,
    pub nick_name: String,
    pub auth_token: String,
    pub token_expires_at: i64,
    pub domain: String,
    pub ab_code: String,
    pub geo_key: String,
    pub privilege: i32,
    pub phone: String,
}
