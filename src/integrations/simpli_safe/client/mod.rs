static _SIMPLI_SAFE_API_URL: &str = "https://api.simplisafe.com";

// pub async fn get_refresh_token() -> Result<TuyaAuthRes, Box<dyn Error>> {
//     let res = request("/v1.0/token?grant_type=1", "").await;
//     let tuya_auth: TuyaAuthRes = serde_json::from_str(&res)?;
//     println!("{:?}", tuya_auth);
//     Ok(tuya_auth)
// }

// pub async fn request(path: &str, token: &str) -> String {}
