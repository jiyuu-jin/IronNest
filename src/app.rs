use leptos::*;

#[server(HandleAssistantCommand)]
pub async fn handle_assistant_command(text: String) -> Result<String, ServerFnError> {
    use {crate::integrations::openai::open_api_command, sqlx::PgPool};
    let pool = use_context::<PgPool>().unwrap();
    open_api_command(text, &pool).await
}
