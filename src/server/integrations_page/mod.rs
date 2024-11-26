use {
    crate::integrations::iron_nest::types::{ControlMessage, Integration},
    leptos::*,
};

#[server(GetIntegrations)]
pub async fn get_integrations() -> Result<Vec<Integration>, ServerFnError> {
    use {
        crate::integrations::iron_nest::types::Integration,
        sqlx::{PgPool, Postgres},
    };

    let pool = use_context::<PgPool>().unwrap();

    let query = "
        SELECT id, name, enabled, image
        FROM integration
        ORDER BY id
    ";
    sqlx::query_as::<Postgres, Integration>(query)
        .fetch_all(&pool)
        .await
        .map_err(Into::into)
}

#[server(ToggleIntegration)]
pub async fn toggle_integration(id: i64, enabled: bool) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>().unwrap();
    // let control_senders =
    //     use_context::<Arc<RwLock<HashMap<String, Sender<ControlMessage>>>>>().unwrap();

    // Update the integration status in the database
    let query = "
        UPDATE integration
        SET enabled = $1
        WHERE id = $2
    ";
    sqlx::query(query)
        .bind(enabled)
        .bind(id)
        .execute(&pool)
        .await?;

    // let senders = control_senders.read().unwrap();
    // if let Some(sender) = senders.get("tplink") {
    //     let message = if enabled {
    //         ControlMessage::Start
    //     } else {
    //         ControlMessage::Stop
    //     };
    //     sender.send(message).await.unwrap();
    // }

    Ok(())
}