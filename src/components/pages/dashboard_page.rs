use crate::components::{
    command_box::CommandBox, device_list::DeviceList, ring_cameras::RingCameras,
};

use {
    crate::integrations::{iron_nest::types::Device, ring::types::RingValues},
    leptos::*,
    std::sync::Arc,
};

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

    view! {
        <main class="lg:pl-20">
            <div class="xl:pl-96">
                <div class="px-4 py-10 sm:px-6 lg:px-8 lg:py-6">
                    <RingCameras ring_values=ring_values />
                    <CommandBox />
                </div>
            </div>
        </main>

        <aside class="fixed inset-y-0 left-20 hidden w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 sm:px-6 lg:px-8 xl:block space-y-0.5">
            <DeviceList ring_values=ring_values />
        </aside>
    }
}
