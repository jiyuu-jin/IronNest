use {crate::integrations::iron_nest::types::FullAction, leptos::*, uuid::Uuid};

#[server(GetActions)]
pub async fn get_actions() -> Result<Vec<FullAction>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    get_actions_query(&pool).await.map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn get_actions_query(pool: &sqlx::PgPool) -> Result<Vec<FullAction>, sqlx::Error> {
    let query = "
        SELECT
            (actions.action->>'id')::UUID AS id,
            (actions.action->>'name') AS name,
            (actions.action->>'cron') AS cron,
            (actions.action->>'function_name') AS function_name,
            (actions.action->>'function_args')::JSONB AS function_args
        FROM
            config,
            LATERAL jsonb_array_elements(data->'actions') AS actions(action)
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
    let function_args = serde_json::from_str::<serde_json::Value>(&function_args).unwrap();
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let cron_client = use_context::<crate::integrations::iron_nest::cron::CronClient>().unwrap();
    let query = r#"
        UPDATE config
        SET data = jsonb_insert(
            data,
            '{actions, -1}',
            jsonb_build_object(
                'id', $1::TEXT,
                'name', $2::TEXT,
                'cron', $3::TEXT,
                'function_name', $4::TEXT,
                'function_args', $5::JSONB
            )
        )
    "#;
    sqlx::query(query)
        .bind(Uuid::new_v4())
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
pub async fn delete_action(id: Uuid) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    delete_action_query(&pool, id).await.map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn delete_action_query(pool: &sqlx::PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    let query = "
        UPDATE config
        SET data = jsonb_set(
            data,
            '{actions}',
            COALESCE(
                (SELECT jsonb_agg(action)
                    FROM jsonb_array_elements(data->'actions') AS action
                    WHERE (action->>'id') <> $1::TEXT),
                '[]'::jsonb
            )
        )
    ";
    sqlx::query(query).bind(id).execute(pool).await.map(|_| ())
}
