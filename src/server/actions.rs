use {
    crate::integrations::iron_nest::types::FullAction,
    leptos::prelude::*,
    serde::{Deserialize, Serialize},
    server_fn::codec::JsonEncoding,
    std::fmt::Display,
    uuid::Uuid,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AddActionError {
    ServerFnError(ServerFnErrorErr),
    ScheduleTasks(String),
    Sql(String),
}

impl FromServerFnError for AddActionError {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        AddActionError::ServerFnError(value)
    }
}

impl Display for AddActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[server(AddAction)]
pub async fn add_action(
    name: String,
    cron: String,
    function_name: String,
    function_args: String,
) -> Result<(), AddActionError> {
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
        .await
        .map_err(|e| AddActionError::Sql(e.to_string()))?;
    cron_client
        .schedule_tasks(&pool)
        .await
        .map_err(|e| AddActionError::ScheduleTasks(e.to_string()))?;
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

#[server(RunAction)]
pub async fn run_action(id: Uuid) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let actions = get_actions_query(&pool).await?;
    let action = actions.iter().find(|a| a.id == id).unwrap();
    crate::integrations::iron_nest::execute_function(
        action.fields.function_name.clone(),
        action.fields.function_args.clone(),
    )
    .await;

    Ok(())
}
