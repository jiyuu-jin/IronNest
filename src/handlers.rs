use std::sync::Arc;
use axum::extract::State;
use crate::utils::RingRestClient;
use crate::types::LocationsRes;

pub async fn ring_handler(State(ring_rest_client): State<Arc<RingRestClient>>) -> String {
   let locations_res = ring_rest_client.get_locations().await;
   let locations = serde_json::from_str::<LocationsRes>(&locations_res).expect(&format!("locations_res: {locations_res}"));
   for location in locations.user_locations {
      println!("{}", location.location_id)
   }

   let devices_res = ring_rest_client.get_devices().await;
   let socket_token = ring_rest_client.get_socket_ticket().await;
   println!("{}",devices_res);
   println!("{}", socket_token);

   let snapshot_res = ring_rest_client.get_camera_snapshot().await;
   return snapshot_res;
}

pub async fn ring_auth_handler(State(ring_rest_client): State<Arc<RingRestClient>>) -> String {
   ring_rest_client.request_auth_token().await
}