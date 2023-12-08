use {
    crate::{
        error_template::{AppError, ErrorTemplate},
        integrations::{
            ring::types::Doorbot,
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
        roku::{discover_roku, get_active_app, send_roku_keypress},
        tplink::{discover_devices, tplink_turn_on, tplink_turn_off},
    };
    cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
        use crate::integrations::{
            roku::{discover_roku, get_active_app, send_roku_keypress},
            tplink::discover_devices,
        };
        use async_openai::{
            types::{
                ChatCompletionFunctionsArgs, ChatCompletionRequestUserMessageArgs,
                CreateChatCompletionRequestArgs,
                ChatCompletionRequestFunctionMessageArgs,
            },
            Client,
        };
    }

    pub enum AssistantFunction {
        RokuKeyPress { key: String },
        TPLinkTurnOn {},
        TPLinkTurnOff {},
    }

    impl AssistantFunction {
        async fn execute(self) -> Result<String, ServerFnError> {
            match self {
                AssistantFunction::RokuKeyPress { key } => {
                    send_roku_keypress(&key).await;
                    Ok(format!("Roku Key Pressed: {}", key))
                }
                AssistantFunction::TPLinkTurnOn {} => {
                    tplink_turn_on().await;
                    Ok(format!("TP-link switch turned on"))
                }
                AssistantFunction::TPLinkTurnOff {} => {
                    tplink_turn_off().await;
                    Ok(format!("TP-link switch turned off"))
                }
            }
        }
    }
}}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Script src="https://cdn.tailwindcss.com"/>
        <Script>
            "window.addEventListener('DOMContentLoaded', () => {
                const toggleButtons = document.querySelectorAll('.toggle-sidebar');
                const sidebar = document.querySelector('.sidebar');
                
                toggleButtons.forEach(button => {
                    button.addEventListener('click', () => {
                        console.log('Toggle sidebar clicked');
                        sidebar.classList.toggle('hidden');
                    });
                });
            });"
        </Script>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no"/>
        <Meta name="apple-mobile-web-app-capable" content="yes"/>
        <Meta name="mobile-web-app-capable" content="yes"/>
        <div>
            <div class="relative z-50 lg:hidden sidebar" role="dialog" aria-modal="true">
                <div class="fixed inset-0 bg-gray-900/80"></div>

                <div class="fixed inset-0 flex">
                    <div class="relative mr-16 flex w-full max-w-xs flex-1">
                        <div class="absolute left-full top-0 flex w-16 justify-center pt-5">
                            <button type="button" class="-m-2.5 p-2.5 toggle-sidebar">
                                <span class="sr-only">Close sidebar</span>
                                <svg
                                    class="h-6 w-6 text-white"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    aria-hidden="true"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M6 18L18 6M6 6l12 12"
                                    ></path>
                                </svg>
                            </button>
                        </div>

                        <div class="flex grow flex-col gap-y-5 overflow-y-auto bg-gray-900 px-6 pb-2 ring-1 ring-white/10">
                            <div class="flex h-16 shrink-0 items-center">
                                <img class="h-8 w-auto" src="/icon.png" alt="Iron Nest"/>
                            </div>
                            <nav class="flex flex-1 flex-col">
                                <ul role="list" class="-mx-2 flex-1 space-y-1">
                                    <li>
                                        <a
                                            href="/"
                                            class="bg-gray-800 text-white group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
                                        >
                                            <svg
                                                class="h-6 w-6 shrink-0"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="currentColor"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25"
                                                ></path>
                                            </svg>
                                            Dashboard
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="#"
                                            class="text-gray-400 hover:text-white hover:bg-gray-800 group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold"
                                        >
                                            <svg
                                                class="h-6 w-6 shrink-0"
                                                fill="none"
                                                viewBox="0 0 24 24"
                                                stroke-width="1.5"
                                                stroke="currentColor"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                    d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z"
                                                ></path>
                                            </svg>
                                            Accounts
                                        </a>
                                    </li>
                                </ul>
                            </nav>
                        </div>
                    </div>
                </div>
            </div>

            <div class="hidden lg:fixed lg:inset-y-0 lg:left-0 lg:z-50 lg:block lg:w-20 lg:overflow-y-auto lg:bg-gray-900 lg:pb-4">
                <div class="flex h-16 shrink-0 items-center justify-center">
                    <img
                        class="h-8 w-auto"
                        src="https://tailwindui.com/img/logos/mark.svg?color=indigo&shade=500"
                        alt="IronNest"
                    />
                </div>
                <nav class="mt-8">
                    <ul role="list" class="flex flex-col items-center space-y-1">
                        <li>
                            <a
                                href="/"
                                class="bg-gray-800 text-white group flex gap-x-3 rounded-md p-3 text-sm leading-6 font-semibold"
                            >
                                <svg
                                    class="h-6 w-6 shrink-0"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    aria-hidden="true"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25"
                                    ></path>
                                </svg>
                                <span class="sr-only">Dashboard</span>
                            </a>
                        </li>
                        <li>
                            <a
                                href="/login"
                                class="text-gray-400 hover:text-white hover:bg-gray-800 group flex gap-x-3 rounded-md p-3 text-sm leading-6 font-semibold"
                            >
                                <svg
                                    class="h-6 w-6 shrink-0"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    aria-hidden="true"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z"
                                    ></path>
                                </svg>
                                <span class="sr-only">Accounts</span>
                            </a>
                        </li>
                        <li>
                            <a
                                href="/"
                                class="text-gray-400 hover:text-white hover:bg-gray-800 group flex gap-x-3 rounded-md p-3 text-sm leading-6 font-semibold"
                            >
                                <svg
                                    class="h-6 w-6 shrink-0"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    aria-hidden="true"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z"
                                    ></path>
                                </svg>
                                <span class="sr-only">Integrations</span>
                            </a>
                        </li>
                    </ul>
                </nav>
            </div>

            <div class="sticky top-0 z-40 flex items-center gap-x-6 bg-gray-900 px-4 py-4 shadow-sm sm:px-6 lg:hidden">
                <button type="button" class="-m-2.5 p-2.5 text-gray-400 lg:hidden toggle-sidebar">
                    <span class="sr-only">Open sidebar</span>
                    <svg
                        class="h-6 w-6"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                        stroke="currentColor"
                        aria-hidden="true"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5"
                        ></path>
                    </svg>
                </button>
                <div class="flex-1 text-sm font-semibold leading-6 text-white">Dashboard</div>
                <a href="#">
                    <span class="sr-only">Your profile</span>
                    <img class="h-8 w-8 rounded-full bg-gray-800" src="/icon.png" alt=""/>
                </a>
            </div>

            <Router fallback=|| {
                let mut outside_errors = Errors::default();
                outside_errors.insert_with_default_key(AppError::NotFound);
                view! { <ErrorTemplate outside_errors/> }.into_view()
            }>
                <main>
                    <Routes>
                        <Route path="/login" view=LoginPage/>
                        <Route path="/" view=DashboardPage/>
                        <Route path="/websocket" view=WebSocketPage/>
                    </Routes>
                </main>
            </Router>
        </div>
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

#[server(HandleAssistantCommand)]
pub async fn handle_assistant_command(text: String) -> Result<String, ServerFnError> {
    println!("calling assistant with {:?}", text);
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("gpt-3.5-turbo-0613")
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(text.to_string())
            .build()?
            .into()])
        .functions([ChatCompletionFunctionsArgs::default()
            .name("send_roku_keypress")
            .description("Send a keypress to a roku device")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "enum": [ "PowerOn", "PowerOff", "home", "rev", "fwd", "play",
                    "select", "left", "right", "down", "up", "back", "replay", "info",
                    "backspace", "enter", "volumeDown", "volumeUp",
                    "volumeMute", "inputTuner", "inputHDMI1", "inputHDMI2",
                    "inputHDMI3", "inputHDMI4", "inputAV1", "channelUp",
                    "channelDown"] },
                },
                "required": ["key"],
            }))
            .build()?])
        .function_call("auto")
        .build()?;

    let response_message = client
        .chat()
        .create(request)
        .await
        .unwrap()
        .choices
        .get(0)
        .unwrap()
        .message
        .clone();

    let value = if let Some(function_call) = response_message.function_call {
        let mut available_functions = HashMap::new();
        available_functions.insert("send_roku_keypress", send_roku_keypress);
        let function_name = function_call.name;
        let function_args: serde_json::Value = function_call.arguments.parse().unwrap();

        let key_press = function_args["key"].as_str().unwrap();

        let function = available_functions.get(function_name.as_str()).unwrap();
        let function_response = function(key_press);

        let message = vec![
            ChatCompletionRequestUserMessageArgs::default()
                .content(text.to_string())
                .build()?
                .into(),
            ChatCompletionRequestFunctionMessageArgs::default()
                .content(function_response.await.to_string())
                .name(function_name)
                .build()?
                .into(),
        ];

        println!("{}", serde_json::to_string(&message).unwrap());

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model("gpt-3.5-turbo-0613")
            .messages(message)
            .build()?;

        let response = client.chat().create(request).await.unwrap();
        let value = format!(
            "{:?}",
            match response.choices[0].message.content {
                None => "No output found!",
                Some(ref x) => x,
            }
        );
        value.to_string()
    } else {
        "".to_string()
    };
    Ok(value)
}

#[component]
fn LoginPage() -> impl IntoView {
    let handle_login = create_server_action::<HandleLogin>();
    let value = handle_login.value();

    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <div class="sm:mx-auto sm:w-full sm:max-w-sm">
                <img
                    class="mx-auto h-20 w-auto"
                    src="https://cdn.shopify.com/s/files/1/2393/8647/files/31291831386201.jpg?v=1701174026"
                    alt="Your Company"
                />
                <h2 class="mt-10 text-center text-2xl font-bold leading-9 tracking-tight text-gray-900">
                    Sign in to your account
                </h2>
            </div>

            <div class="mt-10 sm:mx-auto sm:w-full sm:max-w-sm">
                <ActionForm action=handle_login class="space-y-6">
                    <div>
                        <label for="email" class="block text-sm font-medium leading-6 text-white">
                            Email address
                        </label>
                        <div class="mt-2">
                            <input
                                id="email"
                                name="username"
                                type="text"
                                placeholder="Username"
                                autocomplete="email"
                                required
                                class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            />
                        </div>
                    </div>

                    <div>
                        <div class="flex items-center justify-between">
                            <label
                                for="password"
                                class="block text-sm font-medium leading-6 text-white"
                            >
                                Password
                            </label>
                        </div>
                        <div class="mt-2">
                            <input
                                type="password"
                                name="password"
                                placeholder="Password"
                                autocomplete="current-password"
                                required
                                class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            />
                        </div>
                    </div>
                    <div>
                        <div class="flex items-center justify-between">
                            <label for="tfa" class="block text-sm font-medium leading-6 text-white">
                                Password
                            </label>
                        </div>
                        <div class="mt-2">
                            <input
                                type="password"
                                name="tfa"
                                placeholder="2FA code"
                                class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-indigo-600 sm:text-sm sm:leading-6"
                            />
                        </div>
                    </div>

                    <div>
                        <input
                            type="submit"
                            value="Login"
                            class="flex w-full justify-center rounded-md bg-indigo-500 px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm hover:bg-indigo-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500"
                        />
                    </div>
                </ActionForm>
                <p>{value}</p>
            </div>
        </div>
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

    // let media_text = get_media_player().await;
    // println!("media xml: {}", media_text);

    // get_active_channel().await;

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

    let handle_assistant = create_server_action::<HandleAssistantCommand>();
    let value = handle_assistant.value();

    view! {
        <main class="lg:pl-20">
            <div class="xl:pl-96">
                <div class="px-4 py-10 sm:px-6 lg:px-8 lg:py-6">
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
                        <br/> <ActionForm action=handle_assistant class="space-y-6">
                            <textarea
                                name="text"
                                type="text"
                                class="resize rounded-md border-2 p-2 h-32 w-full border-blue-500"
                                placeholder="Enter text and hit enter"
                            ></textarea>
                            <div class="flex-shrink-0">
                                <button
                                    type="submit"
                                    class="inline-flex items-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                                >
                                    Submit command
                                </button>
                            </div>
                        </ActionForm> {value}
                    </Suspense>
                </div>
            </div>
        </main>

        <aside class="fixed inset-y-0 left-20 hidden w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 sm:px-6 lg:px-8 xl:block space-y-0.5">
            <h2 class="text-lg">"TP-Link Devices"</h2>
            <hr/>
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
            <br/>
            <h2 class="text-lg">"Roku Devices"</h2>
            <hr/>
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
        </aside>
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
