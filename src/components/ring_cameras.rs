use {
    crate::integrations::ring::types::{RingValues, VideoItem},
    leptos::*,
};

#[component]
pub fn RingCameras(ring_values: Resource<(), Result<RingValues, ServerFnError>>) -> impl IntoView {
    let start_of_day_timestamp = get_start_of_day_timestamp();
    let (selected_video_url, set_selected_video_url) = create_signal(String::new());

    view! {
        <Suspense fallback=|| {
            view! { <p>"Loading Ring cameras..."</p> }
        }>
            {move || match ring_values.get() {
                Some(data) => {
                    match data {
                        Ok(data) => {
                            view! {
                                <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;">
                                    {data
                                        .cameras
                                        .iter()
                                        .map(|camera| {
                                            let video_timeline = create_video_timeline(
                                                camera.videos.video_search.clone(),
                                                start_of_day_timestamp,
                                                set_selected_video_url.clone(),
                                            );
                                            view! {
                                                <div>
                                                    <h2>
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

    // Ensure a minimum width (e.g., 2 pixels) for visibility
    let min_width = 5;
    if calculated_width < min_width {
        min_width
    } else {
        calculated_width
    }
}
