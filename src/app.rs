use {leptos::*, std::sync::Arc};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    use crate::integrations::{
        openai::open_api_command,
    };
}}

#[server(HandleLogin)]
pub async fn handle_login(
    username: String,
    password: String,
    tfa: String,
) -> Result<String, ServerFnError> {
    use crate::integrations::ring::client::RingRestClient;
    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let result = ring_rest_client
        .request_auth_token(&username, &password, &tfa)
        .await;

    Ok(result)
}

#[server(HandleAssistantCommand)]
pub async fn handle_assistant_command(text: String) -> Result<String, ServerFnError> {
    use sqlx::{Pool, Sqlite};
    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();
    open_api_command(text, &*pool).await
}
