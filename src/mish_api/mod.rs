use {
    crate::{integrations::iron_nest::AppState, ipld_codecs},
    axum::{extract::State, Json},
    bytes::Bytes,
    cid::Cid,
    ipld_core::codec::Codec,
    multihash_codetable::{Code, MultihashDigest},
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

// pub async fn download_file(cid: String) -> Bytes {
//     todo!()
// }

// pub async fn get_mish_state(name: String) -> MishState {
//     todo!()
// }

// pub async fn set_mish_state(name: String, state: MishState) {

// }
