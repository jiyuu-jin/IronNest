use std::string;

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

#[derive(Deserialize)]
pub struct DoorBotHealth {
    pub battery_percentage: u64,
}

#[derive(Deserialize)]
pub struct AuthorizedDoorbots {
    pub id: u64,
    pub description: String,
    pub health: DoorBotHealth,
}

#[derive(Deserialize)]
pub struct DevicesRes {
    pub authorized_doorbots: Vec<AuthorizedDoorbots>,
}

#[derive(Deserialize)]
pub struct CameraEventsRes {
    pub events: Vec<CameraEvent>,
}

#[derive(Deserialize)]
pub struct CameraEvent {
    pub event_id: String,
    pub event_type: String,
    pub created_at: String,
    pub recorded: Option<bool>,
    pub recording_status: Option<String>,
    pub cv_properties: CvProperties,
}

#[derive(Deserialize)]
pub struct CvProperties {
    pub person_detected: Option<bool>,
}

#[derive(Deserialize)]
pub struct SocketTicketRes {
    pub ticket: String,
}

#[derive(Deserialize)]
pub struct VideoSearchRes {
    pub video_search: Vec<VideoItem>,
}

#[derive(Deserialize)]
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
