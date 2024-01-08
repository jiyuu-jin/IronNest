use {
    super::pages::dashboard_page::DashboardValues, crate::integrations::ring::types::VideoItem,
    leptos::*,
};

#[component]
pub fn RingCameras(
    ring_values: Resource<(), Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    let start_of_day_timestamp = get_start_of_day_timestamp();

    // @TODO learn leptos and fix hardcoded state logic
    let mut signals = Vec::new();
    for _ in 0..2 {
        let (signal, set_signal) = create_signal(String::new());
        signals.push((signal, set_signal));
    }
    let (selected_video_url_1, set_selected_video_url_1) = signals[0];
    let (selected_video_url_2, set_selected_video_url_2) = signals[1];

    view! {
        <Suspense fallback=|| {
            view! {
                <div
                    class="mb-4"
                    style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;"
                >
                    <button
                        style="min-height:360px;"
                        type="button"
                        class="relative block w-full rounded-lg border-2 border-dashed border-gray-300 p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 content-center"
                    >
                        <span class="mt-2 block text-xl font-semibold text-gray-900">
                            "Camera Loading..."
                        </span>
                    </button>
                    <button
                        style="max-height:360px;"
                        type="button"
                        class="relative block w-full rounded-lg border-2 border-dashed border-gray-300 p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
                    >
                        <span class="mt-2 block text-xl font-semibold text-gray-900">
                            "Camera Loading..."
                        </span>
                    </button>
                </div>
            }
        }>
            {move || match ring_values.get() {
                Some(data) => {
                    match data {
                        Ok(data) => {
                            view! {
                                <div
                                    class="mb-4"
                                    style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;"
                                >
                                    {data
                                        .cameras
                                        .iter()
                                        .enumerate()
                                        .map(|(index, camera)| {
                                            let is_index_zero = index.to_string() == '0'.to_string();
                                            let video_timeline = create_video_timeline(
                                                camera.videos.video_search.clone(),
                                                start_of_day_timestamp,
                                                if is_index_zero {
                                                    set_selected_video_url_1.clone()
                                                } else {
                                                    set_selected_video_url_2.clone()
                                                },
                                            );
                                            let selected_video_url = if is_index_zero {
                                                selected_video_url_1
                                            } else {
                                                selected_video_url_2
                                            };
                                            view! {
                                                <div class="rounded-xl shadow-md border border-gray-200">
                                                    <h2 class="p-2">
                                                        {format!(
                                                            "{} - Battery: {}",
                                                            camera.description,
                                                            camera.health,
                                                        )}

                                                    </h2>

                                                    {if !selected_video_url.get().is_empty() {
                                                        view! {
                                                            <div>
                                                                <video
                                                                    style="width: 100%"
                                                                    src=selected_video_url.get().clone()
                                                                    autoplay=true
                                                                    controls=true
                                                                ></video>
                                                            </div>
                                                        }
                                                    } else {
                                                        view! {
                                                            <div>
                                                                <img
                                                                    style="width: 100%"
                                                                    src=format!(
                                                                        "data:image/png;base64,{}",
                                                                        camera.snapshot.image,
                                                                    )
                                                                />

                                                            </div>
                                                        }
                                                    }}

                                                    <p>{"Time: "} {&camera.snapshot.timestamp}</p>
                                                    <div style="max-width: 100%; overflow-x: auto;">
                                                        {video_timeline}
                                                    </div>
                                                </div>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </div>
                            }
                        }
                        Err(_) => {
                            view! {
                                <div
                                    class="mb-2"
                                    style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;"
                                >
                                    <button
                                        style="max-height:360px;"
                                        type="button"
                                        class="relative block w-full rounded-lg border-2 border-dashed border-gray-300 p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
                                    >
                                        <span class="mt-2 block text-xl font-semibold text-gray-900">
                                            "Loading data or none available."
                                        </span>
                                    </button>
                                </div>
                            }
                        }
                    }
                }
                None => {
                    view! {
                        <div
                            class="mb-2"
                            style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;"
                        >
                            <button
                                style="max-height:360px;"
                                type="button"
                                class="relative block w-full rounded-lg border-2 border-dashed border-gray-300 p-12 text-center hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
                            >
                                <span class="mt-2 block text-xl font-semibold text-gray-900">
                                    "Loading data or none available."
                                </span>
                            </button>
                        </div>
                    }
                }
            }}

        </Suspense>
    }
}

fn create_video_timeline(
    videos: Vec<VideoItem>, // Now taking ownership of the data
    start_of_day_timestamp: u64,
    set_selected_video_url: WriteSignal<std::string::String>,
) -> impl IntoView {
    let timeline_width = 1400; // Fixed timeline width in pixels

    view! {
        <div
            class="video-timeline mb-2"
            style=format!(
                "overflow-x: auto; white-space: nowrap; padding: 10px; background: #eee; position: relative; width: {}px; height:25px;",
                timeline_width,
            )
        >

            // Use into_iter() for owned data
            {videos
                .into_iter()
                .map(|video| {
                    let position = calculate_position(
                        video.created_at,
                        start_of_day_timestamp,
                        timeline_width,
                    );
                    let width = calculate_width(video.duration, timeline_width);
                    let video_style = format!(
                        "position: absolute; left: {}px; width: {}px; height: 10px; background-color: #007bff; border-radius: 5px;",
                        position,
                        width,
                    );
                    view! {
                        <a
                            href="javascript:void(0)"
                            style=video_style
                            class="video-duration-pill"
                            on:click=move |_| {
                                set_selected_video_url.set(video.hq_url.clone());
                                ()
                            }
                        >
                        </a>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

fn get_start_of_day_timestamp() -> u64 {
    let now = chrono::Local::now();
    now.date().and_hms(0, 0, 0).timestamp_millis() as u64
}

fn calculate_position(timestamp: u64, start_of_day_timestamp: u64, timeline_width: i32) -> i32 {
    let position = timestamp - start_of_day_timestamp;
    let position_percentage = (position as f64 / 86_400_000f64) * 100.0;
    (position_percentage * timeline_width as f64 / 100.0) as i32
}

fn calculate_width(duration: i32, timeline_width: i32) -> i32 {
    let duration_ms = (duration as u64) * 1000; // Convert duration to milliseconds
    let width_percentage = (duration_ms as f64 / 86_400_000f64) * 100.0; // Calculate width as a percentage of the day
    let calculated_width = (width_percentage * timeline_width as f64 / 100.0) as i32;

    let min_width = 5;
    if calculated_width < min_width {
        min_width
    } else {
        calculated_width
    }
}
