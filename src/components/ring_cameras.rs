use {
    super::pages::dashboard_page::DashboardValues,
    crate::integrations::ring::types::{RingCamera, VideoItem},
    chrono::{DateTime, Utc},
    leptos::*,
};

#[component]
pub fn RingCameras(
    ring_values: Resource<(), Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    let start_of_day_timestamp = get_start_of_day_timestamp();

    view! {
        <div
            class="mb-2"
            style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;"
        >
            <Suspense fallback=|| {
                view! {
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
                }
            }>
                {move || {
                    ring_values
                        .get()
                        .map(|data| {
                            match data {
                                Ok(data) => {
                                    view! {
                                        {data
                                            .cameras
                                            .iter()
                                            .map(|camera| {
                                                camera_component(start_of_day_timestamp, camera.clone())
                                            })
                                            .collect::<Vec<_>>()}
                                    }
                                        .into_view()
                                }
                                Err(e) => {
                                    view! { <p>{format!("RingCameras error: {e}")}</p> }.into_view()
                                }
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}

fn camera_component(start_of_day_timestamp: i64, camera: RingCamera) -> impl IntoView {
    let (selected_video_url, set_selected_video_url) = create_signal(None);

    let video_timeline = create_video_timeline(
        camera.videos.video_search,
        start_of_day_timestamp,
        set_selected_video_url,
    );
    view! {
        <div class="rounded-xl shadow-md border border-gray-200">
            <h2 class="p-2">{format!("{} - Battery: {}", camera.description, camera.health)}</h2>

            {move || match selected_video_url.get() {
                Some(selected_video_url) => {
                    view! {
                        <div>
                            <video
                                style="width: 100%"
                                src=selected_video_url
                                autoplay=true
                                controls=true
                            ></video>
                        </div>
                    }
                }
                None => {
                    view! {
                        <div>
                            <img
                                style="width: 100%"
                                src=format!("data:image/png;base64,{}", camera.snapshot.image)
                            />

                        </div>
                    }
                }
            }}

            <p>{"Time: "} {camera.snapshot.timestamp.to_string()}</p>
            <div style="max-width: 100%; overflow-x: auto;">{video_timeline}</div>
        </div>
    }
}

fn create_video_timeline(
    videos: Vec<VideoItem>, // Now taking ownership of the data
    start_of_day_timestamp: i64,
    set_selected_video_url: WriteSignal<Option<String>>,
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
                                set_selected_video_url.set(Some(video.hq_url.clone()));
                            }
                        >
                        </a>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

/// Gets the timestamp of the start of today in the local timezone
fn get_start_of_day_timestamp() -> i64 {
    chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("Date should be valid with non-invalid params")
        .and_utc()
        .timestamp_millis()
}

fn calculate_position(
    timestamp: DateTime<Utc>,
    start_of_day_timestamp: i64,
    timeline_width: i32,
) -> i32 {
    let start_of_day = DateTime::from_naive_utc_and_offset(
        chrono::NaiveDateTime::from_timestamp_opt(start_of_day_timestamp, 0).unwrap(),
        Utc,
    );
    let position = (timestamp - start_of_day).num_seconds() as f64;
    let position_percentage = (position / 86_400.0) * 100.0;
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
