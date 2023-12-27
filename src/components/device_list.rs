use {crate::integrations::ring::types::RingValues, leptos::*};

#[component]
pub fn DeviceList(ring_values: Resource<(), Result<RingValues, ServerFnError>>) -> impl IntoView {
    view! {
        <h2 class="text-lg">"Devices"</h2>
        <hr/>
        <Suspense fallback=|| {
            view! { <p>"Loading devices..."</p> }
        }>
            {move || {
                ring_values
                    .get()
                    .map(|data| {
                        data.map(|data| {
                            view! {
                                <ul class="tplink-device-list space-y-2">
                                    {data
                                        .devices
                                        .iter()
                                        .map(|device| {
                                            view! {
                                                <li class="tplink-device">
                                                    <div class="device-alias">
                                                        {&device.name} - {format!("({})", &device.device_type)}
                                                    </div>
                                                    <div class="device-name">{&device.ip}</div>
                                                    <div class="device-state">
                                                        {format!("State: {}", &device.state)}
                                                    </div>
                                                </li>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </ul>
                            }
                        })
                    })
            }}

        </Suspense>
    }
}
