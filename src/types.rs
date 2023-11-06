use serde::Deserialize;

#[derive(Deserialize)]
pub struct OauthRes {
  pub hid: String,
  pub rt: String,
}

#[derive(Deserialize)]
pub struct GeoCoordinates {
  pub latitude: f64,
  pub longitude: f64,
}

#[derive(Deserialize)]
pub struct Address {
  pub address1: String,
  pub address2: String, 
  pub cross_street: String,
  pub city: String,
  pub state: String,
  pub zip_code: String,
  pub country: String,
  pub timezone: String,
}

#[derive(Deserialize)]
pub struct UserLocations {
  pub location_id: String,
  pub owner_id: u64,
  pub name: String,
  pub geo_coordinates: GeoCoordinates,
  pub created_at: String,
  pub updated_at: String,
  pub location_type: String,
  pub is_jobsite: bool,
  pub is_owner: bool,
}

#[derive(Deserialize)]
pub struct LocationsRes {
  pub user_locations: Vec<UserLocations>,
}