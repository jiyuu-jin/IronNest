use crate::integrations::ring::types::RingValues;
use crate::{app::HandleAssistantCommand, integrations::iron_nest::types::Device};
use std::sync::Arc;
use {leptos::*, leptos_router::*};

#[server(GetRingValues)]
pub async fn get_ring_values() -> Result<RingValues, ServerFnError> {
    use {
        crate::integrations::ring::{client::RingRestClient, get_ring_camera},
        sqlx::{Pool, Row, Sqlite},
    };

    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();

    let rows = sqlx::query("SELECT id, name, ip, power_state FROM devices")
        .fetch_all(&*pool)
        .await?;

    let mut devices = Vec::new();
    for row in rows {
        devices.push(Device {
            id: row.get("id"),
            name: row.get("name"),
            ip: row.get("ip"),
            state: row.get("power_state"),
        });
    }

    let (mut locations, ring_devices) = tokio::join!(
        ring_rest_client.get_locations(),
        ring_rest_client.get_devices()
    );
    let mut cameras = Vec::with_capacity(20);
    let location = locations.user_locations.remove(0);

    let doorbots = ring_devices
        .doorbots
        .into_iter()
        .chain(ring_devices.authorized_doorbots.into_iter())
        .collect::<Vec<_>>();

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
        devices,
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
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
            <h2 class="text-lg">"Devices"</h2>
            <hr/>
            <Suspense fallback=|| {
                view! { <p>"Loading devices..."</p> }
            }>
                {move || {
                    ring_values
                        .get()
                        .map(|data| {
                            data.map(|data| {
                                view! {
                                    <ul class="tplink-device-list space-y-2">
                                        {data
                                            .devices
                                            .iter()
                                            .map(|device| {
                                                view! {
                                                    <li class="tplink-device">
                                                        <div class="device-alias">{&device.name}</div>
                                                        <div class="device-name">{&device.ip}</div>
                                                        <div class="device-state">
                                                            {format!("State: {}", &device.state)}
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
