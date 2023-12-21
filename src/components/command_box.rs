use {leptos::*, leptos_router::ActionForm, std::sync::Arc};

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

#[component]
pub fn CommandBox() -> impl IntoView {
    let handle_assistant = create_server_action::<HandleAssistantCommand>();
    let value = handle_assistant.value();

    view! {
      <ActionForm action=handle_assistant class="space-y-6">
        <textarea
            name="text"
            type="text"
            class="resize rounded-md border-2 p-2 h-32 w-full border-blue-500"
            placeholder="Enter text and hit enter"
        ></textarea>
        <div class="flex-shrink-0">
            <button
                type="submit"
                class="inline-flex items-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
            >
                Submit command
            </button>
        </div>
      </ActionForm> {value}
    }
}
