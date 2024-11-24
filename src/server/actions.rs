use {crate::integrations::iron_nest::types::FullAction, leptos::*, serde_json::Value};

#[server(GetActions)]
pub async fn get_actions() -> Result<Vec<FullAction>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    get_actions_query(&pool).await.map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn get_actions_query(pool: &sqlx::PgPool) -> Result<Vec<FullAction>, sqlx::Error> {
    let query = "
        SELECT id, name, cron, function_name, function_args
        FROM actions
        ORDER BY name
    ";
    sqlx::query_as(query).fetch_all(pool).await
}

#[server(AddAction)]
pub async fn add_action(
    name: String,
    cron: String,
    function_name: String,
    function_args: String,
) -> Result<(), ServerFnError> {
    let function_args = serde_json::from_str::<Value>(&function_args).unwrap();
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let cron_client = use_context::<crate::integrations::iron_nest::cron::CronClient>().unwrap();
    let query = "
        INSERT INTO actions (name, cron, function_name, function_args)
        VALUES ($1, $2, $3, $4)
    ";
    sqlx::query(query)
        .bind(name)
        .bind(cron)
        .bind(function_name)
        .bind(function_args)
        .execute(&pool)
        .await?;
    cron_client
        .schedule_tasks(&pool)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(())
}

#[server(DeleteAction)]
pub async fn delete_action(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    delete_action_query(&pool, id).await.map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn delete_action_query(pool: &sqlx::PgPool, id: i64) -> Result<(), sqlx::Error> {
    let query = "
        DELETE FROM actions
        WHERE id=$1
    ";
    sqlx::query(query).bind(id).execute(pool).await.map(|_| ())
}
