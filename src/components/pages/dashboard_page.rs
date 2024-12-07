use {
    crate::{
        components::{
            command_box::CommandBox, device_list::DeviceList, planned_meals::PlannedMeals,
            ring_cameras::RingCameras, roku_tv_remote::RokuTvRemote,
        },
        integrations::{ring::types::RingCamera, roku::types::AppsAppWithIcon},
        server::dashboard_page::get_devices,
    },
    leptos::prelude::*,
    serde::{Deserialize, Serialize},
};

cfg_if::cfg_if! {
    if #[cfg(feature = "ssr")] {
        use {
            crate::integrations::ring::types::{
                RingCameraSnapshot, RingVideoRow, VideoItem, VideoSearchRes,
            }
        };
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub name: String,
    pub amount: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScheduledMeal {
    pub recipie_name: String,
    pub ingredients: Ingredient,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DashboardValues {
    pub ws_url: String,
    pub location_name: String,
    pub cameras: Vec<RingCamera>,
    pub roku_apps: Vec<AppsAppWithIcon>,
    pub scheduled_meals: Vec<ScheduledMeal>,
}

#[server(GetDashboardValues)]
pub async fn get_dashboard_values() -> Result<DashboardValues, ServerFnError> {
    use {
        crate::integrations::roku::{roku_get_apps, roku_get_channel_icon},
        sqlx::{PgPool, Postgres, Row},
    };

    let pool = use_context::<PgPool>().unwrap();

    let ring_camera_rows = sqlx::query(
        "SELECT id, description, snapshot_image, snapshot_timestamp, health FROM ring_cameras",
    )
    .fetch_all(&pool)
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

        let ring_videos_res = sqlx::query_as::<Postgres, RingVideoRow>(video_events_query)
            .fetch_all(&pool)
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
        scheduled_meals: vec![ScheduledMeal {
            recipie_name: "Pizza".to_owned(),
            ingredients: Ingredient {
                name: "Doh".to_owned(),
                amount: "1 lb".to_owned(),
            },
        }],
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let dashboard_values = Resource::new(|| (), |_| get_dashboard_values());
    let devices = Resource::new(|| (), |_| get_devices());

    view! {
        <div class="xl:pl-96">
            <div class="px-4 py-10 sm:px-6 lg:px-8 lg:py-6 h-screen">
                <div class="grid lg:grid-cols-12 grid-cols-1 my-4 gap-2 overflow-auto">
                    <RokuTvRemote dashboard_values=dashboard_values/>
                    <PlannedMeals dashboard_values=dashboard_values/>
                    <RingCameras ring_values=dashboard_values/>
                    <CommandBox/>
                </div>
            </div>
        </div>

        <aside class="bg-blue-100 fixed inset-y-0 left-20 hidden w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 sm:px-6 lg:px- xl:block space-y-0.5">
            <DeviceList devices=devices/>
        </aside>
    }
}
