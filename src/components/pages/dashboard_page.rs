use {
    crate::{
        components::{
            command_box::CommandBox, device_list::DeviceList, ring_cameras::RingCameras,
            roku_tv_remote::RokuTvRemote,
        },
        integrations::{
            iron_nest::types::Device,
            ring::types::{
                RingCamera, RingCameraSnapshot, RingVideoRow, VideoItem, VideoSearchRes,
            },
            roku::types::AppsAppWithIcon,
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
    pub roku_apps: Vec<AppsAppWithIcon>,
}

#[server(GetDashboardValues)]
pub async fn get_dashboard_values() -> Result<DashboardValues, ServerFnError> {
    use {
        crate::integrations::roku::{roku_get_apps, roku_get_channel_icon},
        sqlx::{Pool, Row, Sqlite},
    };

    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();

    let ring_camera_rows = sqlx::query(
        "SELECT id, description, snapshot_image, snapshot_timestamp, health FROM ring_cameras",
    )
    .fetch_all(&*pool)
    .await?;

    let apps = roku_get_apps("10.0.0.217").await;
    let mut apps_with_icon = Vec::new();

    for app in apps.apps.into_iter() {
        apps_with_icon.push(AppsAppWithIcon {
            icon: roku_get_channel_icon("10.0.0.217", &app.id).await,
            name: app.name,
            id: app.id,
            app_type: app.app_type,
            version: app.version,
        });
    }

    let mut cameras = Vec::new();
    for ring_camera_row in ring_camera_rows {
        let video_events_query = "
            SELECT ding_id, camera_id, created_at, hq_url
            FROM ring_video_item
        ";

        let ring_videos_res = sqlx::query_as::<Sqlite, RingVideoRow>(video_events_query)
            .fetch_all(&*pool)
            .await
            .unwrap();

        let video_items = ring_videos_res
            .iter()
            .map(|video| VideoItem {
                ding_id: "".to_string(),
                created_at: video.created_at,
                updated_at: 0,
                hq_url: video.hq_url.clone(),
                lq_url: "".to_string(),
                is_e2ee: false,
                manifest_id: None,
                preroll_duration: 0.0,
                thumbnail_url: None,
                untranscoded_url: "".to_string(),
                kind: "".to_string(),
                state: "".to_string(),
                had_subscription: false,
                radar_data_url: None,
                favorite: false,
                duration: 0,
                device_placement: None,
                owner_id: "".to_string(),
            })
            .collect();
        println!("{:?}", video_items);

        cameras.push(RingCamera {
            id: ring_camera_row.get("id"),
            description: ring_camera_row.get("description"),
            snapshot: RingCameraSnapshot {
                image: ring_camera_row.get("snapshot_image"),
                timestamp: ring_camera_row.get("snapshot_timestamp"),
            },
            health: ring_camera_row.get("health"),
            videos: VideoSearchRes {
                video_search: video_items,
            },
        });
    }

    Ok(DashboardValues {
        location_name: "".to_string(),
        cameras,
        ws_url: "".to_string(),
        roku_apps: apps_with_icon,
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let dashboard_values = create_resource(|| (), |_| get_dashboard_values());
    let devices = create_resource(|| (), |_| get_devices());

    view! {
        <main class="lg:pl-20">
            <div class="xl:pl-96">
                <div class="px-4 py-10 sm:px-6 lg:px-8 lg:py-6">
                    <RingCameras ring_values=dashboard_values/>
                    <RokuTvRemote dashboard_values=dashboard_values/>
                    <CommandBox/>
                </div>
            </div>
        </main>

        <aside class="bg-gray-100 fixed inset-y-0 left-20 hidden w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 sm:px-6 lg:px- xl:block space-y-0.5">
            <DeviceList devices=devices/>
        </aside>
    }
}

#[server(GetDevices)]
pub async fn get_devices() -> Result<Vec<Device>, ServerFnError> {
    use {
        crate::integrations::iron_nest::types::Device,
        sqlx::{Pool, Sqlite},
        std::sync::Arc,
    };

    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();

    let query = "
        SELECT id, name, device_type, ip, power_state, 0 AS battery_percentage 
        FROM devices
    ";
    sqlx::query_as::<Sqlite, Device>(query)
        .fetch_all(&*pool)
        .await
        .map_err(Into::into)
}
