use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct OauthRes {
    pub hid: String,
    pub rt: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
}

#[derive(Deserialize, Debug)]
pub struct GeoCoordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct LocationsRes {
    pub user_locations: Vec<UserLocations>,
}

#[derive(Deserialize, Debug)]
pub struct DoorBotHealth {
    pub battery_percentage: u64,
}

#[derive(Deserialize, Debug)]
pub struct Doorbots {
    pub id: u64,
    pub description: String,
    pub health: DoorBotHealth,
}

#[derive(Deserialize, Debug)]
pub struct DevicesRes {
    pub doorbots: Vec<Doorbots>,
    pub authorized_doorbots: Vec<Doorbots>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CameraEventsRes {
    pub events: Vec<CameraEvent>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CameraEvent {
    pub event_id: String,
    pub event_type: String,
    pub created_at: String,
    pub recorded: Option<bool>,
    pub recording_status: Option<String>,
    pub cv_properties: CvProperties,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CvProperties {
    pub person_detected: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct SocketTicketRes {
    pub ticket: String,
}

#[derive(Deserialize, Debug)]
pub struct VideoSearchRes {
    pub video_search: Vec<VideoItem>,
}

#[derive(Deserialize, Debug)]
pub struct VideoItem {
    pub ding_id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub hq_url: String,
    pub lq_url: String,
    pub is_e2ee: bool,
    pub manifest_id: Option<String>,
    pub preroll_duration: f64,
    pub thumbnail_url: Option<String>,
    pub untranscoded_url: String,
    pub kind: String,
    pub state: String,
    pub had_subscription: bool,
    pub radar_data_url: Option<String>,
    pub favorite: bool,
    pub duration: i32,
    pub device_placement: Option<String>,
    pub owner_id: String,
}
