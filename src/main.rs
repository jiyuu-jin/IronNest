mod handlers;
mod types;
mod utils;

use crate::handlers::ring_auth_handler;
use axum::{routing::get, Router};
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use dotenv::dotenv;
use handlers::ring_handler;
use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use types::OauthRes;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let refresh_token = env::var("RING_REFRESH_TOKEN").expect("RING_REFRESH_TOKEN not set");
    let auth_token = env::var("RING_AUTH_TOKEN").expect("RING_REFRESH_TOKEN not set");

    let bytes = base64.decode(refresh_token).unwrap();

    let oauth_res = serde_json::from_slice::<OauthRes>(&bytes).unwrap();
    let ring_rest_client = Arc::new(utils::RingRestClient::new(
        oauth_res.rt,
        oauth_res.hid,
        auth_token,
    ));

    let app = Router::new()
        .route("/", get("Iron Nest is running!"))
        .route("/api/ring", get(ring_handler))
        .route("/api/ring/auth", get(ring_auth_handler))
        .with_state(ring_rest_client);

    axum::Server::bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, 3333)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
