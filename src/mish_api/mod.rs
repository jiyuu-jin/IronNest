use {
    crate::{
        integrations::iron_nest::{AppState, mish::MishStateModification},
        ipld_codecs,
    },
    axum::{Json, extract::State},
    bytes::Bytes,
    cid::Cid,
    ipld_core::codec::Codec,
    jsonpath_rust::{JsonPath, parser::errors::JsonPathError, query::queryable::Queryable},
    multihash_codetable::{Code, MultihashDigest},
    serde::Deserialize,
    serde_ipld_dagjson::codec::DagJsonCodec,
    tokio::sync::mpsc::UnboundedSender,
};

pub async fn upload_dag_json_file(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<String>, ()> {
    let content = DagJsonCodec::encode_to_vec(&body).unwrap();
    let cid = Cid::new_v1(
        <DagJsonCodec as Codec<serde_json::Value>>::CODE,
        Code::Sha2_256.digest(&content),
    );
    let query = "
        INSERT INTO ipld_blobs (cid, content)
        VALUES ($1, $2)
    ";
    sqlx::query(query)
        .bind(cid.to_bytes())
        .bind(content)
        .execute(&state.pool)
        .await
        .unwrap();
    Ok(Json(cid.to_string()))
}

pub async fn upload_raw_file(
    State(state): State<AppState>,
    content: Bytes,
) -> Result<Json<String>, ()> {
    let cid = Cid::new_v1(ipld_codecs::RAW, Code::Sha2_256.digest(&content));
    let query = "
        INSERT INTO ipld_blobs (cid, content)
        VALUES ($1, $2)
        ON CONFLICT (cid) DO NOTHING
    ";
    sqlx::query(query)
        .bind(cid.to_bytes())
        .bind(content.to_vec())
        .execute(&state.pool)
        .await
        .unwrap();
    Ok(Json(cid.to_string()))
}

#[derive(Deserialize)]
pub struct UpdateMishStateBody {
    pub mish_state_name: String,
    pub path: String,
    pub content: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct MishStateRow {
    state: serde_json::Value,
}

pub async fn update_mish_state_handler(
    State(state): State<AppState>,
    Json(body): Json<UpdateMishStateBody>,
) -> Result<(), String> {
    update_mish_state(&state.pool, &state.mish_state_modification_bus_sender, body)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn update_mish_state(
    pool: &sqlx::PgPool,
    mish_state_modification_bus_sender: &UnboundedSender<MishStateModification>,
    body: UpdateMishStateBody,
) -> Result<(), anyhow::Error> {
    let query = "
        SELECT state
        FROM mish_states
        WHERE name = $1
    ";
    let row = sqlx::query_as::<_, MishStateRow>(query)
        .bind(&body.mish_state_name)
        .fetch_optional(pool)
        .await?;

    if let Some(mut mish_state) = row {
        update_json_via_jsonpath(&mut mish_state.state, &body.path, &body.content)?;

        // Update the state in the database
        let update_query = "
            UPDATE mish_states
            SET state = $1
            WHERE name = $2
        ";
        sqlx::query(update_query)
            .bind(&mish_state.state)
            .bind(&body.mish_state_name)
            .execute(pool)
            .await?;
        mish_state_modification_bus_sender.send(MishStateModification::CreateOrUpdate {
            name: body.mish_state_name,
            state: mish_state.state,
        })?;
    } else {
        // If the state doesn't exist, create a new one
        let mut new_state = serde_json::json!({});
        update_json_via_jsonpath(&mut new_state, &body.path, &body.content)?;
        let insert_query = "
            INSERT INTO mish_states (name, state)
            VALUES ($1, $2)
        ";
        sqlx::query(insert_query)
            .bind(&body.mish_state_name)
            .bind(&new_state)
            .execute(pool)
            .await?;
        mish_state_modification_bus_sender.send(MishStateModification::CreateOrUpdate {
            name: body.mish_state_name,
            state: new_state,
        })?;
    }
    Ok(())
}

fn update_json_via_jsonpath(
    state: &mut serde_json::Value,
    path: &str,
    content: &serde_json::Value,
) -> Result<(), JsonPathError> {
    let result = state.query_only_path(path)?;
    for item in result {
        let state = state.reference_mut(item);
        if let Some(state) = state {
            *state = content.clone();
        }
    }
    Ok(())
}
