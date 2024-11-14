use {crate::integrations::iron_nest::types::Action, leptos::*};

#[server(GetActions)]
pub async fn get_actions() -> Result<Vec<Action>, ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>().unwrap();

    let query = "
        SELECT id, name
        FROM actions
        ORDER BY name
    ";
    sqlx::query_as(query)
        .fetch_all(&pool)
        .await
        .map_err(Into::into)
}

#[server(AddAction)]
pub async fn add_action(name: String) -> Result<(), ServerFnError> {
    use sqlx::PgPool;

    let pool = use_context::<PgPool>().unwrap();

    let query = "
        INSERT INTO actions (name)
        VALUES ($1)
    ";
    sqlx::query(query).bind(name).execute(&pool).await?;

    Ok(())
}
