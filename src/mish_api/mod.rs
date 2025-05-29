use {
    crate::{
        integrations::iron_nest::{mish::MishStateModification, AppState},
        ipld_codecs,
    },
    axum::{extract::State, Json},
    bytes::Bytes,
    cid::Cid,
    ipld_core::codec::Codec,
    multihash_codetable::{Code, MultihashDigest},
    serde::Deserialize,
    serde_ipld_dagjson::codec::DagJsonCodec,
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
    mish_state_name: String,
    path: String,
    content: serde_json::Value,
}

#[derive(sqlx::FromRow)]
struct MishStateRow {
    state: serde_json::Value,
}

pub async fn update_mish_state(
    State(state): State<AppState>,
    Json(body): Json<UpdateMishStateBody>,
) {
    let query = "
        SELECT state
        FROM mish_states
        WHERE name = $1
    ";
    let row = sqlx::query_as::<_, MishStateRow>(query)
        .bind(&body.mish_state_name)
        .fetch_optional(&state.pool)
        .await
        .unwrap();

    if let Some(mut mish_state) = row {
        update_json_at_path(&mut mish_state.state, &body.path, &body.content);

        // Update the state in the database
        let update_query = "
            UPDATE mish_states
            SET state = $1
            WHERE name = $2
        ";
        sqlx::query(update_query)
            .bind(&mish_state.state)
            .bind(&body.mish_state_name)
            .execute(&state.pool)
            .await
            .unwrap();
        state
            .mish_state_modification_bus_sender
            .send(MishStateModification::CreateOrUpdate {
                name: body.mish_state_name,
                state: mish_state.state,
            })
            .unwrap();
    } else {
        // If the state doesn't exist, create a new one
        let mut new_state = serde_json::json!({});
        update_json_at_path(&mut new_state, &body.path, &body.content);
        let insert_query = "
            INSERT INTO mish_states (name, state)
            VALUES ($1, $2)
        ";
        sqlx::query(insert_query)
            .bind(&body.mish_state_name)
            .bind(&new_state)
            .execute(&state.pool)
            .await
            .unwrap();
        state
            .mish_state_modification_bus_sender
            .send(MishStateModification::CreateOrUpdate {
                name: body.mish_state_name,
                state: new_state,
            })
            .unwrap();
    }
}

fn update_json_at_path(state: &mut serde_json::Value, path: &str, content: &serde_json::Value) {
    let path_parts: Vec<&str> = path.split('.').collect();
    let mut current = state.as_object_mut().unwrap();
    for (i, part) in path_parts.iter().enumerate() {
        if i == path_parts.len() - 1 {
            current.insert(part.to_string(), content.clone());
        } else {
            current = current
                .entry(part.to_string())
                .or_insert_with(|| serde_json::json!({}))
                .as_object_mut()
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_json_at_path_existing() {
        let mut state = serde_json::json!({
            "a": {
                "b": 42
            }
        });
        let content = serde_json::json!(100);
        update_json_at_path(&mut state, "a.b", &content);
        assert_eq!(
            state,
            serde_json::json!({
                "a": {
                    "b": 100
                }
            })
        );
    }

    #[test]
    fn test_update_json_at_path_new() {
        let mut state = serde_json::json!({});
        let content = serde_json::json!("new value");
        update_json_at_path(&mut state, "x.y.z", &content);
        assert_eq!(
            state,
            serde_json::json!({
                "x": {
                    "y": {
                        "z": "new value"
                    }
                }
            })
        );
    }

    #[test]
    fn test_update_json_at_path_single_element() {
        let mut state = serde_json::json!({});
        let content = serde_json::json!("single");
        update_json_at_path(&mut state, "single", &content);
        assert_eq!(
            state,
            serde_json::json!({
                "single": "single"
            })
        );
    }

    #[test]
    fn test_update_json_at_path_zero_element() {
        let mut state = serde_json::json!({});
        let content = serde_json::json!("root");
        update_json_at_path(&mut state, "", &content);
        assert_eq!(
            state,
            serde_json::json!({
                "": "root"
            })
        );
    }
}
