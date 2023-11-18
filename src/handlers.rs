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
    let back_camera_recordings = ring_rest_client.get_recordings(back_device_id).await;

    let front_camera_component = camera_recordings_list(front_camera_recordings);
    let back_camera_component = camera_recordings_list(back_camera_recordings);

    ring_rest_client
        .subscribe_to_motion_events(front_device_id)
        .await;

    let html_text = format!(
        r#"<html>
            <head>
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <meta name="apple-mobile-web-app-capable" content="yes">
                <meta name="mobile-web-app-capable" content="yes">
                <link rel="manifest" href="/manifest.json">
            </head>
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
                    console.log("registering service worker 1")
                    if ('serviceWorker' in navigator && 'Notification' in window) {{
                        console.log("registering service worker 2")
                        navigator.serviceWorker.register('/service-worker.js')
                            .then(registration => {{
                                console.log('Service Worker registered');
                                // Request notification permission on page load
                                Notification.requestPermission().then(permission => {{
                                    console.log({{permission}});
                                    if (permission === 'granted') {{
                                        registration.active.postMessage({{ type: 'NOTIFY' }});
                                        setInterval(() => {{
                                            console.log("hh");
                                            registration.active.postMessage({{ type: 'NOTIFY' }});
                                        }}, 60000); // 60000ms = 1 minute
                                    }}
                                }});
                            }})
                            .catch(error => {{
                                console.error('Service Worker registration failed:', error);
                            }});
                    }}                    
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

pub async fn roku_handler() -> String {
    "Hello Roku".to_string()
}
