use {
    crate::error_template::{AppError, ErrorTemplate},
    base64::{engine::general_purpose::STANDARD as base64, Engine},
    js_sys::Reflect,
    leptos::*,
    leptos_meta::*,
    leptos_reactive::create_effect,
    leptos_router::*,
    leptos_use::{core::ConnectionReadyState, use_websocket, UseWebsocketReturn},
    serde::{Deserialize, Serialize},
    serde_json::json,
    wasm_bindgen::{closure::Closure, JsValue},
    web_sys::RtcPeerConnection,
};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/{{project-name}}.css"/>

        // sets the document title
        <Meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no"/>
        <Meta name="apple-mobile-web-app-capable" content="yes"/>
        <Meta name="mobile-web-app-capable" content="yes"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors/> }.into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="/login" view=LoginPage/>
                    <Route path="/ring" view=RingPage/>
                    <Route path="/websocket" view=WebSocketPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <h1>"Iron Nest is Running"</h1>
        <p>
            <a href="/login">Login</a>
        </p>
        <p>
            <A href="/ring">"Ring"</A>
        </p>
        <p>
            <a href="/api/roku" rel="external">
                "Roku"
            </a>
        </p>
        <p>
            <A href="/websocket">"WebSocket"</A>
        </p>
    }
}

#[server(HandleLogin)]
pub async fn handle_login(
    username: String,
    password: String,
    tfa: String,
) -> Result<String, ServerFnError> {
    use {crate::integrations::ring::RingRestClient, std::sync::Arc};

    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let result = ring_rest_client
        .request_auth_token(&username, &password, &tfa)
        .await;

    Ok(result)
}

#[component]
fn LoginPage() -> impl IntoView {
    let handle_login = create_server_action::<HandleLogin>();
    let value = handle_login.value();

    view! {
        <h1>"Login"</h1>
        <ActionForm action=handle_login>
            <input type="text" name="username" placeholder="Username"/>
            <input type="password" name="password" placeholder="Password"/>
            <input type="password" name="tfa" placeholder="2FA code"/>
            <input type="submit" value="Login"/>
        </ActionForm>
        <p>{value}</p>
        <p>
            <A href="/">"Home"</A>
        </p>
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RingValues {
    pub ws_url: String,
    pub front_camera: RingCamera,
    pub back_camera: RingCamera,
    pub location_name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RingCameraSnapshot {
    pub image: String,
    pub timestamp: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RingCamera {
    pub id: u64,
    pub description: String,
    pub snapshot: RingCameraSnapshot,
    pub health: u64,
}

#[server(GetRingValues)]
pub async fn get_ring_values() -> Result<RingValues, ServerFnError> {
    use {crate::integrations::ring::RingRestClient, std::sync::Arc};

    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let mut locations = ring_rest_client.get_locations().await;
    let devices = ring_rest_client.get_devices().await;

    let back_snapshot_res = ring_rest_client.get_camera_snapshot("375458730").await;
    let back_image_base64 = base64.encode(back_snapshot_res.1);

    let front_snapshot_res = ring_rest_client.get_camera_snapshot("141328255").await;
    let front_image_base64 = base64.encode(front_snapshot_res.1);

    let location = locations.user_locations.remove(0);

    // let location_id = &location.location_id;
    let mut doorbots = devices
        .doorbots
        .into_iter()
        .chain(devices.authorized_doorbots.into_iter())
        .collect::<Vec<_>>();

    let back_camera = doorbots.remove(0);
    let front_camera = doorbots.remove(0);

    // let front_camera_events = ring_rest_client
    //     .get_camera_events(location_id, &front_camera.id)
    //     .await;

    // let back_camera_events = ring_rest_client
    //     .get_camera_events(location_id, &back_camera.id)
    //     .await;

    let ws_url = ring_rest_client.get_ws_url().await;

    Ok(RingValues {
        location_name: location.name,
        front_camera: RingCamera {
            id: front_camera.id,
            description: front_camera.description,
            snapshot: RingCameraSnapshot {
                image: front_image_base64,
                timestamp: front_snapshot_res.0,
            },
            health: front_camera.health.battery_percentage,
        },
        back_camera: RingCamera {
            id: back_camera.id,
            description: back_camera.description,
            snapshot: RingCameraSnapshot {
                image: back_image_base64,
                timestamp: back_snapshot_res.0,
            },
            health: back_camera.health.battery_percentage,
        },
        ws_url,
    })
}

#[component]
fn RingPage() -> impl IntoView {
    let ring_values = create_resource(|| (), |_| get_ring_values());

    view! {
        <h1>"Dashboard"</h1>
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || {
                ring_values
                    .get()
                    .map(|data| {
                        data.map(|data| {
                            view! {
                                <Title text="Dashboard"/>
                                <h1>{data.location_name}</h1>
                                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, max-content)); grid-gap: 8px">
                                    <div>
                                        <h2>
                                            {data.front_camera.description} - Battery:
                                            {data.front_camera.health}
                                        </h2>
                                        <img
                                            style="width: 100%"
                                            src=format!(
                                                "data:image/png;base64,{}",
                                                data.front_camera.snapshot.image,
                                            )
                                        />

                                        <h2>Time: {data.front_camera.snapshot.timestamp}</h2>
                                        <h2>Events:</h2>
                                        <ul>
                                            <li>{} - {}</li>
                                        </ul>
                                        <h2>Recordings</h2>

                                        {}
                                    </div>
                                    <div>
                                        <h2>
                                            {data.back_camera.description} - Battery:
                                            {data.back_camera.health}
                                        </h2>
                                        <img
                                            style="width: 100%"
                                            src=format!(
                                                "data:image/png;base64,{}",
                                                data.back_camera.snapshot.image,
                                            )
                                        />

                                        <h2>Time: {data.back_camera.snapshot.timestamp}</h2>
                                        <h2>Events:</h2>
                                        <ul>
                                            <li>{} - {}</li>
                                        </ul>
                                        <h2>Recordings</h2>

                                        {}
                                    </div>
                                </div>
                                <br/>
                                <hr/>
                                <div>Socket Ticket: {data.ws_url}</div>
                            }
                        })
                    })
            }}

        </Suspense>
    }
}

#[component]
fn RokuPage() -> impl IntoView {
    view! {
        <Title text="Roku Remote"/>
        <div>
            <div id="buttons">
                <button class="top-button" onclick="sendCommand('Back')">
                    Back
                </button>
                <button class="top-button" onclick="sendCommand('Home')">
                    Home
                </button>
                <button class="top-button" onclick="sendCommand('PowerOff')">
                    Power
                </button>
                <button class="d-pad-button d-pad-up" onclick="sendCommand('Up')">
                    Up
                </button>
                <button class="d-pad-button d-pad-left" onclick="sendCommand('Left')">
                    Left
                </button>
                <button class="ok-button" onclick="sendCommand('Select')">
                    OK
                </button>
                <button class="d-pad-button d-pad-right" onclick="sendCommand('Right')">
                    Right
                </button>
                <button class="d-pad-button d-pad-down" onclick="sendCommand('Down')">
                    Down
                </button>
                <button class="bottom-button" onclick="sendCommand('Rev')">
                    Rev
                </button>
                <button class="bottom-button" onclick="sendCommand('Play')">
                    Play
                </button>
                <button class="bottom-button" onclick="sendCommand('Fwd')">
                    Fwd
                </button>
            </div>
            <div style="display: flex; justify-content: center;">
                <select id="device-select">
                    <option value="1">Device 1</option>
                    <option value="2">Device 2</option>
                </select>
            </div>
        </div>
    }
}

#[component]
fn WebSocketPage() -> impl IntoView {
    let ring_values = create_resource(|| (), |_| get_ring_values());

    view! {
        <h1>"WebSocket"</h1>
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || {
                ring_values
                    .get()
                    .map(|ring_values| {
                        ring_values
                            .map(|ring_values| {
                                view! { <WebSocketComponent ring_values=ring_values/> }
                            })
                    })
            }}

        </Suspense>
    }
}

#[component]
fn WebSocketComponent(ring_values: RingValues) -> impl IntoView {
    let UseWebsocketReturn {
        ready_state,
        message,
        message_bytes,
        send,
        send_bytes,
        open,
        close,
        ..
    } = use_websocket(&ring_values.ws_url);

    let send_message = move |_| {
        send("Hello, world!");
    };

    // let send_byte_message = move |_| {
    //     send_bytes(b"Hello, world!\r\n".to_vec());
    // };

    create_effect(move |_| {
        if ready_state.get() == ConnectionReadyState::Open {
            let send_bytes = send_bytes.clone();
            let pc = RtcPeerConnection::new().unwrap();
            let create_offer_callback = Closure::wrap(Box::new(move |offer: JsValue| {
                let sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
                    .unwrap()
                    .as_string()
                    .unwrap();
                send_bytes(
                    serde_json::to_vec(&json!({
                        "method": "live_view",
                        "dialog_id": "333333",
                        "body": {
                            "doorbot_id": ring_values.front_camera.id,
                            "stream_options": { "audio_enabled": true, "video_enabled": true },
                            "sdp": sdp,
                        }
                    }))
                    .unwrap(),
                );
            }) as Box<dyn FnMut(JsValue)>);
            let _ = pc.create_offer().then(&create_offer_callback);
            create_offer_callback.forget();
        }
    });
    let status = move || ready_state.get().to_string();

    let connected = move || ready_state.get() == ConnectionReadyState::Open;

    let open_connection = move |_| {
        open();
    };

    let close_connection = move |_| {
        close();
    };

    view! {
        <div>
            <p>"status: " {status}</p>

            <button on:click=send_message disabled=move || !connected()>
                "Send"
            </button>
            // <button on:click=send_byte_message disabled=move || !connected()>
            // "Send bytes"
            // </button>
            <button on:click=open_connection disabled=connected>
                "Open"
            </button>
            <button on:click=close_connection disabled=move || !connected()>
                "Close"
            </button>

            <h2>"Receive message: "</h2>
            <pre style="text-wrap: wrap; word-break: break-all;">
                {move || format!("{:?}", message.get())}
            </pre>
            <p>"Receive byte message: " {move || format!("{:?}", message_bytes.get())}</p>
        </div>
    }
}
