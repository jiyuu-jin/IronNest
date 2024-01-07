use {
    crate::{
        components::{command_box::CommandBox, device_list::DeviceList, ring_cameras::RingCameras},
        integrations::{
            iron_nest::types::Device,
            ring::types::{RingCamera, RingCameraSnapshot, VideoSearchRes},
        },
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
        crate::integrations::iron_nest::types::Device,
        sqlx::{Pool, Row, Sqlite},
    };

    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();

    let query = "
        SELECT id, name, device_type, ip, power_state, battery_percentage
        FROM devices
        ORDER BY name
    ";
    let devices = sqlx::query_as::<Sqlite, Device>(query)
        .fetch_all(&*pool)
        .await?;

    let ring_camera_rows = sqlx::query(
        "SELECT id, description, snapshot_image, snapshot_timestamp, health FROM ring_cameras",
    )
    .fetch_all(&*pool)
    .await?;

    let mut cameras = Vec::new();
    for ring_camera_row in ring_camera_rows {
        cameras.push(RingCamera {
            id: ring_camera_row.get("id"),
            description: ring_camera_row.get("description"),
            snapshot: RingCameraSnapshot {
                image: ring_camera_row.get("snapshot_image"),
                timestamp: ring_camera_row.get("snapshot_timestamp"),
            },
            health: ring_camera_row.get("health"),
            videos: VideoSearchRes {
                video_search: Vec::new(),
            },
        });
    }

    Ok(DashboardValues {
        location_name: "".to_string(),
        cameras,
        ws_url: "".to_string(),
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

        <aside class="bg-gray-100 fixed inset-y-0 left-20 hidden w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 sm:px-6 lg:px-4 xl:block space-y-0.5">
            <DeviceList ring_values=dashboard_values/>
        </aside>
    }
}
