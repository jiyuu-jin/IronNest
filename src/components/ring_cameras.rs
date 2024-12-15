use {
    crate::integrations::ring::types::{RingCamera, VideoItem},
    chrono::{DateTime, Utc},
    leptos::prelude::*,
};

#[component]
pub fn RingCameraPanel(camera: RingCamera) -> impl IntoView {
    let start_of_day_timestamp = get_start_of_day_timestamp();

    let (selected_video_url, set_selected_video_url) = create_signal(None::<String>);
    let video_timeline = create_video_timeline(
        camera.videos.video_search.clone(),
        start_of_day_timestamp,
        set_selected_video_url,
    );

    view! {
        <div class="col-span-3 h-[264px] flex flex-col rounded-lg shadow-md border border-gray-200 bg-white overflow-hidden text-black h-[248px]">
            <div class="flex justify-between px-2 py-2 bg-gray-100">
                <div class="text-sm font-bold">{camera.description}</div>
                <div class="text-sm">{format!("Battery: {}", camera.health)}</div>
            </div>

            <div class="flex-1 flex items-center justify-center overflow-hidden">
                {move || match selected_video_url.get() {
                    Some(selected_video_url) => {
                        view! {
                            <video
                                src=selected_video_url
                                autoplay=true
                                controls=true
                                class="h-full w-full object-contain rounded-lg"
                            ></video>
                        }
                            .into_any()
                    }
                    None => {
                        view! {
                            <img
                                src=format!("data:image/png;base64,{}", camera.snapshot.image)
                                class="h-full w-auto max-h-full object-contain rounded-lg"
                            />
                        }
                            .into_any()
                    }
                }}

            </div>

            <div class="text-xs text-center bg-gray-50 border-t">
                {camera.snapshot.timestamp.to_string()}
            </div>
        </div>
    }
}

fn get_start_of_day_timestamp() -> i64 {
    chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("Date should be valid with non-invalid params")
        .and_utc()
        .timestamp_millis()
}

fn create_video_timeline(
    videos: Vec<VideoItem>,
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

fn calculate_position(
    timestamp: DateTime<Utc>,
    start_of_day_timestamp: i64,
    timeline_width: i32,
) -> i32 {
    let start_of_day = DateTime::from_utc(
        chrono::NaiveDateTime::from_timestamp_millis(start_of_day_timestamp).unwrap(),
        Utc,
    );
    let position = (timestamp - start_of_day).num_seconds() as f64;
    let position_percentage = (position / 86_400.0) * 100.0;
    (position_percentage * timeline_width as f64 / 100.0) as i32
}

fn calculate_width(duration: i32, timeline_width: i32) -> i32 {
    let duration_ms = (duration as u64) * 1000; // Convert duration to milliseconds
    let width_percentage = (duration_ms as f64 / 86_400_000f64) * 100.0; // width as a percentage of a day
    let calculated_width = (width_percentage * timeline_width as f64 / 100.0) as i32;

    let min_width = 5;
    if calculated_width < min_width {
        min_width
    } else {
        calculated_width
    }
}
