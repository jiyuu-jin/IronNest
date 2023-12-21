use {crate::integrations::ring::types::RingValues, leptos::*};

#[component]
pub fn RingCameras(ring_values: Resource<(), Result<RingValues, ServerFnError>>) -> impl IntoView {
    view! {
        <Suspense fallback=|| {
            view! { <p>"Loading Ring cameras..."</p> }
        }>
            {move || {
                match ring_values.get() {
                    Some(data) => {
                        match data {
                            Ok(data) => {
                                view! {
                                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 10px;">
                                        {data
                                            .cameras
                                            .iter()
                                            .map(|camera| {
                                                view! {
                                                    <div>
                                                        <h2>
                                                            {format!(
                                                                "{} - Battery: {}",
                                                                camera.description,
                                                                camera.health,
                                                            )}

                                                        </h2>
                                                        <img
                                                            style="width: 100%"
                                                            src=format!(
                                                                "data:image/png;base64,{}",
                                                                camera.snapshot.image,
                                                            )
                                                        />

                                                        <p>{"Time: "} {&camera.snapshot.timestamp}</p>
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
                }
            }}
            <br/>
        </Suspense>
    }
}
