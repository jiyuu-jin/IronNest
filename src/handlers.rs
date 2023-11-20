use crate::integrations::ring::{camera_recordings_list, RingRestClient};

use {
    crate::integrations::roku::discover_roku,
    axum::{
        extract::{Path, State},
        http::StatusCode,
        response::Html,
    },
    base64::{engine::general_purpose::STANDARD as base64, Engine},
    std::sync::Arc,
};

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
                <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
                <meta name="apple-mobile-web-app-capable" content="yes">
                <meta name="mobile-web-app-capable" content="yes">
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

pub async fn roku_keypress_handler(
    Path((device_id, key)): Path<(i64, String)>,
) -> Result<String, StatusCode> {
    let roku_ip = if device_id == 1 {
        "10.0.0.162"
    } else {
        "10.0.0.217"
    };

    let roku_url = format!("http://{}:8060/keypress/{}", roku_ip, key);
    let client = reqwest::Client::new();

    match client.post(&roku_url).send().await {
        Ok(_) => Ok(format!("Key pressed: {}", key)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn roku_handler() -> Html<String> {
    discover_roku().await;

    let color_value = "#333";
    let html_text = format!(
        r#"<html>
            <head>
                <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
                <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
                <meta name="theme-color" content="{color_value}">
                <link rel="manifest" href="/manifest.json">
                <title>Roku Remote</title>
                <style>
                    body {{
                        font-family: Arial, sans-serif;
                        background: #333;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                        margin: 0;
                        overflow: hidden;
                    }}
                    * {{
                        touch-action: none;
                    }}
                    #buttons {{
                        display: grid;
                        grid-template-columns: 80px 80px 80px;
                        grid-template-rows: 80px auto auto auto auto 80px auto auto auto auto 80px;
                        grid-gap: 15px;
                        background: black;
                        padding: 20px;
                        border-radius: 20px;
                        box-shadow: 0 0 20px rgba(0,0,0,0.5);
                        transform: scale(0.8);
                    }}
                    button {{
                        background-color: #6A0DAD;
                        color: white;
                        border: none;
                        border-radius: 50%;
                        font-size: 16px;
                        cursor: pointer;
                        transition: background-color 0.3s;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        text-align: center;
                        width: 80px;
                        height: 80px;
                    }}
                    .top-button {{
                        grid-column: span 1;
                    }}
                    .top-button:nth-child(1) {{
                        grid-row: 1;
                        grid-column: 1;
                    }}
                    .top-button:nth-child(2) {{
                        grid-row: 1;
                        grid-column: 2;
                    }}
                    .top-button:nth-child(3) {{
                        grid-row: 1;
                        grid-column: 3;
                    }}
                    .d-pad-button, .ok-button {{
                        width: 80px;
                        height: 80px;
                        border-radius: 14px;
                    }}
                    .d-pad-up {{
                        grid-column: 2;
                        grid-row: 4;
                    }}
                    .d-pad-left {{
                        grid-column: 1;
                        grid-row: 5;
                    }}
                    .ok-button {{
                        grid-column: 2;
                        grid-row: 5;
                    }}
                    .d-pad-right {{
                        grid-column: 3;
                        grid-row: 5;
                    }}
                    .d-pad-down {{
                        grid-column: 2;
                        grid-row: 6;
                    }}
                    .bottom-button {{
                        grid-column: span 1;
                        width: 80px;
                        height: 80px;
                        border-radius: 14px;
                    }}
                    /* Specific placement for bottom buttons */
                    .bottom-button:nth-child(9) {{ /* Rev button */
                        grid-row: 8;
                        grid-column: 1;
                    }}
                    .bottom-button:nth-child(10) {{ /* Play button */
                        grid-row: 8;
                        grid-column: 2;
                    }}
                    .bottom-button:nth-child(11) {{ /* Fwd button */
                        grid-row: 8;
                        grid-column: 3;
                    }}
                    button:hover {{
                        background-color: #7B1FA2;
                    }}
                </style>
            </head>
            <body>
                <div>
                    <div id="buttons">
                        <button class="top-button" onclick="sendCommand('Back')">Back</button>
                        <button class="top-button" onclick="sendCommand('Home')">Home</button>
                        <button class="top-button" onclick="sendCommand('PowerOff')">Power</button>
                        <button class="d-pad-button d-pad-up" onclick="sendCommand('Up')">Up</button>
                        <button class="d-pad-button d-pad-left" onclick="sendCommand('Left')">Left</button>
                        <button class="ok-button" onclick="sendCommand('Select')">OK</button>
                        <button class="d-pad-button d-pad-right" onclick="sendCommand('Right')">Right</button>
                        <button class="d-pad-button d-pad-down" onclick="sendCommand('Down')">Down</button>
                        <button class="bottom-button" onclick="sendCommand('Rev')">Rev</button>
                        <button class="bottom-button" onclick="sendCommand('Play')">Play</button>
                        <button class="bottom-button" onclick="sendCommand('Fwd')">Fwd</button>
                    </div>
                    <div style="display: flex; justify-content: center;">
                        <select id="device-select">
                            <option value="1">Device 1</option>
                            <option value="2">Device 2</option>
                        </select>
                    </div>
                </div>
                <script>
                    document.addEventListener('touchstart', function(event) {{
                        if (event.touches.length > 1) {{
                            event.preventDefault();
                        }}
                    }}, {{ passive: false }});

                    document.addEventListener('touchend', function(event) {{
                        if (event.touches.length > 1) {{
                            event.preventDefault();
                        }}
                    }}, {{ passive: false }});

                    document.addEventListener('dblclick', function(event) {{
                        event.preventDefault();
                    }}, {{ passive: false }});

                    function sendCommand(command) {{
                        const deviceId = document.getElementById('device-select').value;
                        const endpoint = `/rest-api/roku/${{deviceId}}/keypress/${{command}}`;
                        fetch(endpoint, {{ method: 'GET' }})
                            .then(response => response.text())
                            .then(data => console.log(data))
                            .catch(error => console.error('Error:', error));
                    }}
                </script>
            </body>
        </html>"#
    );
    Html(html_text)
}
