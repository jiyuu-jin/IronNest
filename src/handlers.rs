use crate::types::{CameraEventsRes, DevicesRes, LocationsRes, VideoSearchRes};
use crate::utils::{camera_recordings_list, RingRestClient};
use axum::extract::State;
use axum::response::Html;
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use std::sync::Arc;

pub async fn ring_handler(State(ring_rest_client): State<Arc<RingRestClient>>) -> Html<String> {
    let locations_res = ring_rest_client.get_locations().await;
    let locations = serde_json::from_str::<LocationsRes>(&locations_res)
        .expect(&format!("locations_res: {locations_res}"));

    let devices_res = ring_rest_client.get_devices().await;
    let devices = serde_json::from_str::<DevicesRes>(&devices_res)
        .expect(&format!("locations_res: {locations_res}"));

    let socket_ticket = ring_rest_client.get_ws_url().await;

    let back_snapshot_res = ring_rest_client.get_camera_snapshot("375458730").await;
    let back_image_base64 = base64.encode(back_snapshot_res.1);

    let front_snapshot_res = ring_rest_client.get_camera_snapshot("141328255").await;
    let front_image_base64 = base64.encode(front_snapshot_res.1);

    let location_id = &locations.user_locations[1].location_id;
    let front_device_id = &devices.authorized_doorbots[1].id;
    let back_device_id = &devices.authorized_doorbots[0].id;

    let front_camera_events_res = ring_rest_client
        .get_camera_events(&location_id, &front_device_id)
        .await;
    let front_camera_events = serde_json::from_str::<CameraEventsRes>(&front_camera_events_res)
        .expect(&format!("camera_event_res: {locations_res}"));

    let back_camera_events_res = ring_rest_client
        .get_camera_events(&location_id, &back_device_id)
        .await;
    let back_camera_events = serde_json::from_str::<CameraEventsRes>(&back_camera_events_res)
        .expect(&format!("camera_event_res: {locations_res}"));

    let front_camera_recordings_res = ring_rest_client.get_recordings(&front_device_id).await;
    let front_camera_recordings =
        serde_json::from_str::<VideoSearchRes>(&front_camera_recordings_res)
            .expect(&format!("camera_event_res: {front_camera_recordings_res}"));

    let front_camera_component = camera_recordings_list(front_camera_recordings);

    let back_camera_recordings_res = ring_rest_client.get_recordings(&back_device_id).await;
    let back_camera_recordings =
        serde_json::from_str::<VideoSearchRes>(&back_camera_recordings_res)
            .expect(&format!("camera_event_res: {back_camera_recordings_res}"));

    let back_camera_component = camera_recordings_list(back_camera_recordings);

    let html_text = format!(
        r#"<html>
         <body>
            <h1>{} - {}</h1>
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, max-content)); grid-gap: 8px">
                <div>
                    <h2>{} - Battery: {}</h2>
                    <img style="width: 100%" src="data:image/png;base64,{front_image_base64}" />
                    <h2>Time: {}</h2>
                    <h2>Events:</h2>
                    <ul>
                        <li>
                        {} - {}
                        </li>
                    </ul>
                    <h2>Recordings</h2>
                    {front_camera_component}
                </div>
                <div>
                    <h2>{} - Battery: {} </h2>
                    <img style="width: 100%" src="data:image/png;base64,{back_image_base64}" />
                    <h2>Time: {}</h2>
                    <h2>Events:</h2>
                    <ul>
                        <li>
                        {} - {}
                        </li>
                    </ul>
                    <h2>Recordings</h2>
                    {back_camera_component}
                </div>
            </div>
            <br />
            <hr />
            <div>Socket Ticket: {socket_ticket}</div>
            <script>
                const webSocket = new WebSocket("{socket_ticket}");
                webSocket.addEventListener("open", event => {{
                    webSock.send(JSON.stringify({{
                        method: 'live_view',
                        dialog_id: '333333',
                        body: {{
                          doorbot_id: {front_device_id},
                          stream_options: {{ audio_enabled: true, video_enabled: true }},
                          sdp,
                        }},
                      }}))
                }});
                console.log({{webSocket}});
            </script>
        </body>
      <html>
   "#,
        &locations.user_locations[0].name,
        &locations.user_locations[1].name,
        &devices.authorized_doorbots[1].description,
        &devices.authorized_doorbots[1].health.battery_percentage,
        front_snapshot_res.0,
        &front_camera_events.events[0].event_type,
        &front_camera_events.events[0].created_at,
        &devices.authorized_doorbots[0].description,
        &devices.authorized_doorbots[0].health.battery_percentage,
        back_snapshot_res.0,
        &back_camera_events.events[0].event_type,
        &back_camera_events.events[0].created_at,
    );
    Html(html_text)
}

pub async fn ring_auth_handler(State(ring_rest_client): State<Arc<RingRestClient>>) -> String {
    ring_rest_client.request_auth_token().await
}
