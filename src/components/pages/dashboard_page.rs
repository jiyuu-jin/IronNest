use {
    crate::{
        components::{
            command_box::CommandBox, device_list::DeviceList, device_panel::DeviceListPanel,
            planned_meals::PlannedMeals, ring_cameras::RingCameraPanel,
            roku_tv_remote::RokuTvRemote,
        },
        integrations::{
            instacart::types::{Ingredient, ScheduledMeal},
            ring::types::RingCamera,
            roku::types::AppsAppWithIcon,
        },
        server::dashboard_page::get_devices,
    },
    leptos::prelude::*,
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
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

    // @TODO add error handling and make roku apps dynamic from roku devices in device table
    // let apps = roku_get_apps("10.0.0.217").await;
    // let mut apps_with_icon = Vec::new();

    // for app in apps.apps.into_iter() {
    //     apps_with_icon.push(AppsAppWithIcon {
    //         icon: roku_get_channel_icon("10.0.0.217", &app.id).await,
    //         name: app.name,
    //         id: app.id,
    //         app_type: app.app_type,
    //         version: app.version,
    //     });
    // }

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
        roku_apps: Vec::new(),
        scheduled_meals: vec![
            ScheduledMeal {
                recipie_name: "Pancakes & Eggs".to_owned(),
                ingredients: Ingredient {
                    name: "Doh".to_owned(),
                    amount: "1 lb".to_owned(),
                },
            },
            ScheduledMeal {
                recipie_name: "Grilled Chicken Salad".to_owned(),
                ingredients: Ingredient {
                    name: "Doh".to_owned(),
                    amount: "1 lb".to_owned(),
                },
            },
            ScheduledMeal {
                recipie_name: "Spaghetti Bolognese".to_owned(),
                ingredients: Ingredient {
                    name: "Doh".to_owned(),
                    amount: "1 lb".to_owned(),
                },
            },
            ScheduledMeal {
                recipie_name: "French Toast & Sausage".to_owned(),
                ingredients: Ingredient {
                    name: "Doh".to_owned(),
                    amount: "1 lb".to_owned(),
                },
            },
            ScheduledMeal {
                recipie_name: "Turkey Club Sandwich".to_owned(),
                ingredients: Ingredient {
                    name: "Doh".to_owned(),
                    amount: "1 lb".to_owned(),
                },
            },
            ScheduledMeal {
                recipie_name: "Grilled Salmon with Quinoa".to_owned(),
                ingredients: Ingredient {
                    name: "Doh".to_owned(),
                    amount: "1 lb".to_owned(),
                },
            },
        ],
    })
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let dashboard_values = Resource::new(|| (), |_| async { get_dashboard_values().await });
    let devices = Resource::new(|| (), |_| get_devices());

    #[derive(Clone, PartialEq)]
    struct PanelData {
        inner: RwSignal<PanelDataInner>,
    }

    #[derive(Clone, PartialEq)]
    struct PanelDataInner {
        component_type: String,
        camera_id: Option<String>,
        device_ids: Option<Vec<i64>>,
    }

    let panel_map = RwSignal::new({
        let map = [
            (
                "roku1".to_string(),
                PanelDataInner {
                    component_type: "roku".to_string(),
                    camera_id: None,
                    device_ids: None,
                },
            ),
            (
                "meals".to_string(),
                PanelDataInner {
                    component_type: "meals".to_string(),
                    camera_id: None,
                    device_ids: None,
                },
            ),
            (
                "command".to_string(),
                PanelDataInner {
                    component_type: "command".to_string(),
                    camera_id: None,
                    device_ids: None,
                },
            ),
            (
                "ring1".to_string(),
                PanelDataInner {
                    component_type: "ring".to_string(),
                    camera_id: Some("375458730".to_string()),
                    device_ids: None,
                },
            ),
            (
                "ring2".to_string(),
                PanelDataInner {
                    component_type: "ring".to_string(),
                    camera_id: Some("141328255".to_string()),
                    device_ids: None,
                },
            ),
            (
                "toggles".to_string(),
                PanelDataInner {
                    component_type: "toggles".to_string(),
                    camera_id: None,
                    device_ids: Some(vec![85, 88, 89, 90]),
                },
            ),
            (
                "toggles2".to_string(),
                PanelDataInner {
                    component_type: "toggles".to_string(),
                    camera_id: None,
                    device_ids: Some(vec![95, 86, 87, 97, 96, 84]),
                },
            ),
        ]
        .into_iter()
        .map(|(id, inner)| {
            (
                id,
                PanelData {
                    inner: RwSignal::new(inner.clone()),
                },
            )
        })
        .collect::<HashMap<_, _>>();
        println!("Initial panel_map: {:?}", map.keys().collect::<Vec<_>>());
        map
    });

    let component_order = RwSignal::new(vec![
        "ring2".to_string(),
        "roku1".to_string(),
        "meals".to_string(),
        "ring1".to_string(),
        "command".to_string(),
        "toggles".to_string(),
        "toggles2".to_string(),
    ]);

    // let add_panel = {
    //     move |panel_type: String, assigned_device_ids: Option<Vec<i64>>| {
    //         let new_panel_id = format!("new_panel_{}", component_order.get().len());
    //         component_order.update(|order| {
    //             order.push(new_panel_id.clone());
    //         });
    //         panel_map.update(|map| {
    //             map.insert(
    //                 new_panel_id.clone(),
    //                 PanelData {
    //                     inner: RwSignal::new(PanelDataInner {
    //                         component_type: panel_type.clone(),
    //                         camera_id: None,
    //                         device_ids: assigned_device_ids,
    //                     }),
    //                 },
    //             );
    //             println!(
    //                 "Updated panel_map after adding new panel: {:?}",
    //                 map.keys().collect::<Vec<_>>()
    //             );
    //         });
    //     }
    // };

    let (sidebar_visible, set_sidebar_visible) = signal(false);
    let toggle_sidebar = move |_| {
        let current = sidebar_visible.get();
        set_sidebar_visible.set(!current);
    };

    let handle_device_click = move |device_id: i64| {
        let target_panel_id = "toggles".to_string();
        panel_map.update(|map| {
            if let Some(panel_data) = map.get_mut(&target_panel_id) {
                panel_data
                    .inner
                    .update(|panel_data| match &mut panel_data.device_ids {
                        Some(ids) => {
                            if !ids.contains(&device_id) {
                                ids.push(device_id);
                                leptos::logging::log!(
                                    "Added device ID {} to panel '{}'",
                                    device_id,
                                    target_panel_id
                                );
                            } else {
                                leptos::logging::log!(
                                    "Device ID {} already exists in panel '{}'",
                                    device_id,
                                    target_panel_id
                                );
                            }
                        }
                        None => {
                            panel_data.device_ids = Some(vec![device_id]);
                            println!(
                                "Initialized device_ids and added device ID {} to panel '{}'",
                                device_id, target_panel_id
                            );
                        }
                    });
            } else {
                println!(
                    "Panel '{}' not found. Creating a new panel and adding device ID {}.",
                    target_panel_id, device_id
                );
                map.insert(
                    target_panel_id.clone(),
                    PanelData {
                        inner: RwSignal::new(PanelDataInner {
                            component_type: "toggles".to_string(),
                            camera_id: None,
                            device_ids: Some(vec![device_id]),
                        }),
                    },
                );
            }
        });
    };
    let callback = Callback::new(handle_device_click);

    let sorted_panels = move || {
        let component_order = component_order.get();
        let mut entries = panel_map.get().into_iter().collect::<Vec<_>>();
        entries.sort_by(|(a, _), (b, _)| {
            component_order
                .iter()
                .position(|x| x == a)
                .unwrap()
                .cmp(&component_order.iter().position(|x| x == b).unwrap())
        });
        entries
    };

    view! {
        <Suspense fallback=|| {
            view! {
                <button
                    type="button"
                    class="relative block w-full rounded-lg border-2 border-dashed border-gray-300 p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 content-center"
                >
                    <span class="mt-2 block text-xl font-semibold text-gray-900">
                        "Dashboard Data Loading..."
                    </span>
                </button>
            }
                .into_any()
        }>
            {move || {
                dashboard_values
                    .get()
                    .map(|data| {
                        match data {
                            Ok(data) => {
                                println!(
                                    "DashboardValues cameras: {:?}",
                                    data.cameras.iter().map(|c| c.id).collect::<Vec<_>>(),
                                );
                                view! {
                                    <div class="px-4 py-10 lg:px-8 lg:py-6 flex">
                                        <Show
                                            when=move || sidebar_visible.get()
                                            fallback=|| view! { <></> }
                                        >
                                            <aside class="ml-20 bg-blue-100 fixed inset-y-0 left-0 w-96 overflow-y-auto border-r border-gray-200 px-4 py-6 space-y-0.5 transition-transform duration-300 ease-in-out">
                                                <DeviceList devices=devices on_device_click=callback/>
                                            </aside>
                                        </Show>
                                        <div class="absolute top-4 left-4">
                                            <button
                                                type="button"
                                                class="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500"
                                                on:click=toggle_sidebar
                                            >
                                                {move || {
                                                    if sidebar_visible.get() { "Hide" } else { "Show" }
                                                }}

                                            </button>
                                        </div>
                                        <div class=move || {
                                            if sidebar_visible.get() {
                                                "ml-96 w-full transition-all duration-300 ease-in-out"
                                            } else {
                                                "w-full transition-all duration-300 ease-in-out"
                                            }
                                        }>
                                            <div class="grid lg:grid-cols-12 grid-cols-1 my-4 gap-x-4 gap-y-4 overflow-hidden">
                                                <For
                                                    each=sorted_panels
                                                    key=|panel| panel.0.clone()
                                                    children=move |(id, panel_data)| {
                                                        let data = data.clone();
                                                        view! {
                                                            {move || {
                                                                leptos::logging::log!("for: id: {id}");
                                                                match panel_data.inner.get().component_type.as_str() {
                                                                    "roku" => {
                                                                        view! { <RokuTvRemote dashboard_values=dashboard_values/> }
                                                                            .into_any()
                                                                    }
                                                                    "meals" => {
                                                                        view! { <PlannedMeals dashboard_values=dashboard_values/> }
                                                                            .into_any()
                                                                    }
                                                                    "command" => view! { <CommandBox/> }.into_any(),
                                                                    "ring" => {
                                                                        let camera_id = panel_data
                                                                            .inner
                                                                            .get()
                                                                            .camera_id
                                                                            .unwrap_or_default();
                                                                        println!("{:?}", data.cameras.first().map(|c| c.id));
                                                                        println!("Looking for camera with ID: {}", camera_id);
                                                                        let camera = data
                                                                            .cameras
                                                                            .iter()
                                                                            .find(|c| c.id.to_string() == camera_id);
                                                                        match camera {
                                                                            Some(camera) => {
                                                                                println!("Found camera: {:?}", camera.id);
                                                                                view! { <RingCameraPanel camera=camera.clone()/> }
                                                                                    .into_any()
                                                                            }
                                                                            None => {
                                                                                println!("Camera with ID {} not found", camera_id);
                                                                                view! {
                                                                                    <div class="bg-gray-200 h-full flex items-center justify-center text-gray-600">
                                                                                        "Camera Not Found"
                                                                                    </div>
                                                                                }
                                                                                    .into_any()
                                                                            }
                                                                        }
                                                                    }
                                                                    "toggles" => {
                                                                        if let Some(device_ids) = panel_data
                                                                            .inner
                                                                            .get()
                                                                            .device_ids
                                                                            .clone()
                                                                        {
                                                                            view! {
                                                                                <DeviceListPanel devices=devices device_ids=device_ids/>
                                                                            }
                                                                                .into_any()
                                                                        } else {
                                                                            view! {
                                                                                <div class="bg-gray-200 h-full flex items-center justify-center text-gray-600">
                                                                                    "No Devices Assigned"
                                                                                </div>
                                                                            }
                                                                                .into_any()
                                                                        }
                                                                    }
                                                                    _ => {
                                                                        view! {
                                                                            <div class="bg-gray-200 h-full flex items-center justify-center text-gray-600">
                                                                                "New Blank Panel"
                                                                            </div>
                                                                        }
                                                                            .into_any()
                                                                    }
                                                                }
                                                            }}
                                                        }
                                                    }
                                                />

                                            </div>
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                println!("Error loading dashboard values: {}", e);
                                view! {
                                    <p class="text-red-500">
                                        {format!("Error loading dashboard: {e}")}
                                    </p>
                                }
                                    .into_any()
                            }
                        }
                    })
            }}

        </Suspense>
    }
}
