use crate::integrations::ring::types::Doorbot;

use {
    crate::{
        error_template::{AppError, ErrorTemplate},
        integrations::{
            roku::types::{ActionApp, RokuDiscoverRes},
            tplink::types::TPLinkDiscoveryData,
        },
    },
    base64::{engine::general_purpose::STANDARD as base64, Engine},
    js_sys::Reflect,
    leptos::*,
    leptos_meta::*,
    leptos_reactive::create_effect,
    leptos_router::*,
    leptos_use::{core::ConnectionReadyState, use_websocket, UseWebsocketReturn},
    serde::{Deserialize, Serialize},
    serde_json::json,
    std::sync::Arc,
    wasm_bindgen::{closure::Closure, JsValue},
    web_sys::RtcPeerConnection,
};

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
    use crate::integrations::{
        roku::{get_media_player, discover_roku, get_active_app, get_active_channel},
        tplink::discover_devices,
    };
}}

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
                    <Route path="/dashboard" view=DashboardPage/>
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
            <A href="/dashboard">"Dashboard"</A>
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
    use crate::integrations::ring::client::RingRestClient;
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
    pub location_name: String,
    pub cameras: Vec<RingCamera>,
    pub tplink_devices: Vec<TPLinkDiscoveryData>,
    pub roku_devices: Vec<RokuDiscoverRes>,
    pub roku_app: ActionApp,
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
    use crate::integrations::ring::client::RingRestClient;
    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let mut locations = ring_rest_client.get_locations().await;
    let devices = ring_rest_client.get_devices().await;
    let mut cameras = Vec::with_capacity(20);
    let location = locations.user_locations.remove(0);

    let doorbots = devices
        .doorbots
        .into_iter()
        .chain(devices.authorized_doorbots.into_iter())
        .collect::<Vec<_>>();

    let tplink_devices = discover_devices().await.unwrap();
    let roku_devices = discover_roku().await;

    let roku_app = get_active_app().await;
    println!("xml {}", roku_app.app[0].value);

    let media_text = get_media_player().await;
    println!("media xml: {}", media_text);

    get_active_channel().await;

    pub async fn get_ring_camera(
        ring_rest_client: &Arc<RingRestClient>,
        device: &Doorbot,
    ) -> RingCamera {
        let device_string = device.id.to_string();
        let snapshot_res = ring_rest_client.get_camera_snapshot(&device_string).await;
        let image_base64 = base64.encode(snapshot_res.1);

        RingCamera {
            id: device.id,
            description: device.description.to_string(),
            snapshot: RingCameraSnapshot {
                image: image_base64,
                timestamp: snapshot_res.0,
            },
            health: device.health.battery_percentage,
        }
    }

    for doorbot in doorbots.iter() {
        cameras.push(get_ring_camera(&ring_rest_client, doorbot).await)
    }
    // let front_camera_events = ring_rest_client
    //     .get_camera_events(location_id, &front_camera.id)
    //     .await;

    // let back_camera_events = ring_rest_client
    //     .get_camera_events(location_id, &back_camera.id)
    //     .await;

    let ws_url = "".to_string();

    Ok(RingValues {
        location_name: location.name,
        cameras,
        ws_url,
        tplink_devices,
        roku_devices,
        roku_app,
    })
}

#[component]
fn DashboardPage() -> impl IntoView {
    let ring_values = create_resource(|| (), |_| get_ring_values());

    view! {
        <div
            class="dashboard-container"
            style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px;"
        >
            <div class="sidebar" style="padding: 10px;">
                <h2>"TP-Link Devices"</h2>
                <Suspense fallback=|| {
                    view! { <p>"Loading TP-Link devices..."</p> }
                }>
                    {move || {
                        ring_values
                            .get()
                            .map(|data| {
                                data.map(|data| {
                                    view! {
                                        <ul class="tplink-device-list">
                                            {data
                                                .tplink_devices
                                                .iter()
                                                .map(|device| {
                                                    view! {
                                                        <li class="tplink-device">
                                                            <div class="device-alias">{&device.alias}</div>
                                                            <div class="device-name">{&device.dev_name}</div>
                                                            <div class="device-state">
                                                                {format!("State: {}", &device.relay_state)}
                                                            </div>
                                                        </li>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    }
                                })
                            })
                    }}

                </Suspense>
                <h2>"Roku Devices"</h2>
                <Suspense fallback=|| {
                    view! { <p>"Loading Roku devices..."</p> }
                }>
                    {move || {
                        ring_values
                            .get()
                            .map(|data| {
                                data.map(|data| {
                                    view! {
                                        <ul class="roku-device-list">
                                            {data
                                                .roku_devices
                                                .iter()
                                                .map(|device| {
                                                    view! {
                                                        <li class="roku-device">
                                                            <div class="device-info">
                                                                {"Location: "} {&device.location} <br/> {"App: "}
                                                                {&data.roku_app.app[0].value}
                                                            </div>
                                                        </li>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    }
                                })
                            })
                    }}

                </Suspense>
            </div>
            <div class="dashboard-main" style="padding: 10px;">
                <h2>"Ring Cameras"</h2>
                <Suspense fallback=|| {
                    view! { <p>"Loading Ring cameras..."</p> }
                }>
                    {move || {
                        match ring_values.get() {
                            Some(data) => {
                                match data {
                                    Ok(data) => {
                                        view! {
                                            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;">
                                                {data
                                                    .cameras
                                                    .iter()
                                                    .map(|camera| {
                                                        view! {
                                                            <div>
                                                                <h2>
                                                                    {format!(
                                                                        "{} - Battery: {}",
                                                                        camera.description,
                                                                        camera.health,
                                                                    )}
                                                                </h2>
                                                                <img
                                                                    style="width: 100%"
                                                                    src=format!(
                                                                        "data:image/png;base64,{}",
                                                                        camera.snapshot.image,
                                                                    )
                                                                />
                                                                <p>{"Time: "} {&camera.snapshot.timestamp}</p>
                                                            </div>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            </div>
                                        }
                                    }
                                    Err(_) => {
                                        view! {
                                            <div>
                                                <p>"Error loading cameras."</p>
                                            </div>
                                        }
                                    }
                                }
                            }
                            None => {
                                view! {
                                    <div>
                                        <p>"Loading data or none available."</p>
                                    </div>
                                }
                            }
                        }
                    }}

                </Suspense>
            </div>
        </div>
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
                            "doorbot_id": "",
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
