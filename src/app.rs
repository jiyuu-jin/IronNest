use {leptos::*, std::sync::Arc};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    use crate::integrations::{
        openai::open_api_command,
    };
}}

#[server(HandleAssistantCommand)]
pub async fn handle_assistant_command(text: String) -> Result<String, ServerFnError> {
    use sqlx::{Pool, Sqlite};
    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();
    open_api_command(text, &*pool).await
}
