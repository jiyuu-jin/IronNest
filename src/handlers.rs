use {
    crate::utils::{camera_recordings_list, RingRestClient},
    axum::{extract::State, response::Html},
    base64::{engine::general_purpose::STANDARD as base64, Engine},
    std::sync::Arc,
};

#[axum::debug_handler]
pub async fn ring_handler(State(ring_rest_client): State<Arc<RingRestClient>>) -> Html<String> {
    let locations = ring_rest_client.get_locations().await;
    let devices = ring_rest_client.get_devices().await;

    let socket_ticket = ring_rest_client.get_ws_url().await;

    let back_snapshot_res = ring_rest_client.get_camera_snapshot("375458730").await;
    let back_image_base64 = base64.encode(back_snapshot_res.1);

    let front_snapshot_res = ring_rest_client.get_camera_snapshot("141328255").await;
    let front_image_base64 = base64.encode(front_snapshot_res.1);

    let location_index = 0;

    let location_id = &locations.user_locations[location_index].location_id;
    let doorbots = devices
        .doorbots
        .iter()
        .chain(devices.authorized_doorbots.iter())
        .collect::<Vec<_>>();
    let front_device_id = &doorbots[1].id;
    let back_device_id = &doorbots[0].id;

    let front_camera_events = ring_rest_client
        .get_camera_events(location_id, front_device_id)
        .await;

    let back_camera_events = ring_rest_client
        .get_camera_events(location_id, back_device_id)
        .await;

    let front_camera_recordings = ring_rest_client.get_recordings(front_device_id).await;
    let front_camera_component = camera_recordings_list(front_camera_recordings);
    let back_camera_recordings = ring_rest_client.get_recordings(back_device_id).await;
    let back_camera_component = camera_recordings_list(back_camera_recordings);

    let html_text = format!(
        r#"<html>
         <body>
            <h1>{}</h1>
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
        &locations.user_locations[location_index].name,
        &doorbots[1].description,
        &doorbots[1].health.battery_percentage,
        front_snapshot_res.0,
        &front_camera_events.events[0].event_type,
        &front_camera_events.events[0].created_at,
        &doorbots[0].description,
        &doorbots[0].health.battery_percentage,
        back_snapshot_res.0,
        &back_camera_events.events[0].event_type,
        &back_camera_events.events[0].created_at,
    );
    Html(html_text)
}

pub async fn ring_auth_handler(State(ring_rest_client): State<Arc<RingRestClient>>) -> String {
    let username = "";
    let password = "";
    ring_rest_client
        .request_auth_token(username, password, "")
        .await
}
