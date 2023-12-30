use {
    crate::{
        components::{command_box::CommandBox, device_list::DeviceList, ring_cameras::RingCameras},
        integrations::{iron_nest::types::Device, ring::types::RingCamera},
    },
    leptos::*,
    serde::{Deserialize, Serialize},
    std::sync::Arc,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct DashboardValues {
    pub ws_url: String,
    pub location_name: String,
    pub cameras: Vec<RingCamera>,
    pub devices: Vec<Device>,
}

#[server(GetDashboardValues)]
pub async fn get_dashboard_values() -> Result<DashboardValues, ServerFnError> {
    use {
        crate::integrations::{
            iron_nest::types::Device,
            ring::{client::RingRestClient, get_ring_camera},
        },
        sqlx::{Pool, Row, Sqlite},
    };

    let ring_rest_client = use_context::<Arc<RingRestClient>>().unwrap();
    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();

    let rows = sqlx::query("SELECT id, name, device_type, ip, power_state FROM devices")
        .fetch_all(&*pool)
        .await?;

    let mut devices = Vec::new();
    for row in rows {
        let state_value: u8 = row.get("power_state");
        let state: u8 = state_value.try_into().expect("Value out of range for u8");
        // let battery_percentage_value: i64 = row.get("battery_percentage");
        // let battery_percentage: u64 = battery_percentage_value
        //     .try_into()
        //     .expect("Value out of range for u64");

        devices.push(Device {
            id: row.get("id"),
            name: row.get("name"),
            device_type: row.get("device_type"),
            ip: row.get("ip"),
            state,
            battery_percentage: 0,
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

    let ws_url = "".to_string();

    Ok(DashboardValues {
        location_name: location.name,
        cameras,
        ws_url,
        devices,
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let dashboard_values = create_resource(|| (), |_| get_dashboard_values());

    view! {
        <main class="lg:pl-20">
            <div class="xl:pl-96">
                <div class="px-4 py-10 sm:px-6 lg:px-8 lg:py-6">
                    <RingCameras ring_values=dashboard_values/>
                    <CommandBox/>
                </div>
            </div>
        </main>

        <aside class="bg-gray-100 fixed inset-y-0 left-20 hidden w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 sm:px-6 lg:px-8 xl:block space-y-0.5">
            <DeviceList ring_values=dashboard_values/>
        </aside>
    }
}
