use {
    super::pages::dashboard_page::DashboardValues, crate::integrations::iron_nest::types::Device,
    leptos::*,
};

#[component]
pub fn DeviceList(
    ring_values: Resource<(), Result<DashboardValues, ServerFnError>>,
) -> impl IntoView {
    view! {
        <div>
            <h2 class="text-lg">"Devices"</h2>
            <hr class="mb-2"/>
            <Suspense fallback=|| {
                view! { <p>"Loading devices..."</p> }
            }>
                {move || {
                    ring_values
                        .get()
                        .map(|data| {
                            data.map(|data| {
                                view! {
                                    <ul class="device-list space-y-2">
                                        {data
                                            .devices
                                            .into_iter()
                                            .map(|device| {
                                                view! { <DeviceListItem device=device/> }
                                            })
                                            .collect::<Vec<_>>()}
                                    </ul>
                                }
                            })
                        })
                }}

            </Suspense>
        </div>
    }
}

pub enum DeviceType {
    SmartPlug,
    SmartLight,
    RingDoorbell,
}

impl DeviceType {
    fn from_str(device_type: &str) -> Option<DeviceType> {
        match device_type {
            "smart-plug" => Some(DeviceType::SmartPlug),
            "smart-light" => Some(DeviceType::SmartLight),
            "ring-doorbell" => Some(DeviceType::RingDoorbell),
            _ => None,
        }
    }
}

#[component]
pub fn DeviceListItem(device: Device) -> impl IntoView {
    if let Some(device_type) = DeviceType::from_str(&device.device_type) {
        match device_type {
            DeviceType::SmartPlug => view! { <SmartPlugItem device=device/> },
            DeviceType::SmartLight => view! { <SmartLightItem device=device/> },
            DeviceType::RingDoorbell => view! { <SmartPlugItem device=device/> },
        }
    } else {
        view! { <SmartPlugItem device=device/> }
    }
}

#[server(HandleSmartPlugToggle)]
pub async fn handle_smart_plug_toggle(state: u8, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on};
    if state == 0 {
        tplink_turn_plug_on(&ip).await;
    } else {
        tplink_turn_plug_off(&ip).await;
    }
    Ok(())
}

#[component]
pub fn SmartPlugItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let state = device.state;

    view! {
        <li class="rounded-lg bg-white shadow p-2">
            <div class="device-alias">{&device.name} {format!("({})", &device.device_type)}</div>
            <div class="device-name">{&device.ip}</div>
            <div class="device-state">
                {format!("State: {}", &device.state)}
                <label class="relative inline-flex items-center cursor-pointer ml-2 mt-2">
                    <input
                        type="checkbox"
                        value=""
                        on:click=move |_| {
                            let ip_clone = ip.clone();
                            let state_clone = state.clone();
                            println!("clicked!");
                            spawn_local(async move {
                                handle_smart_plug_toggle(state_clone.clone(), ip_clone)
                                    .await
                                    .unwrap();
                            });
                        }

                        checked=device.state == 1
                        class="sr-only peer"
                    />
                    <div class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                </label>
            </div>
        </li>
    }
}

#[server(HandleSmartLightToggle)]
pub async fn handle_smart_light_toggle(state: u8, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_turn_light_on_off;
    tplink_turn_light_on_off(&ip, if state == 1 { 0 } else { 1 }).await;
    Ok(())
}

#[component]
pub fn SmartLightItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let state = device.state;

    view! {
        <li class="rounded-lg bg-white shadow p-2">
            <div class="device-alias">{&device.name} {format!("({})", &device.device_type)}</div>
            <div class="device-name">{&device.ip}</div>
            <div class="device-state">
                {format!("State: {}", &device.state)}
                <label class="relative inline-flex items-center cursor-pointer ml-2 mt-2">
                    <input
                        type="checkbox"
                        value=""
                        on:click=move |_| {
                            let ip_clone = ip.clone();
                            let state_clone = state.clone();
                            println!("clicked!");
                            spawn_local(async move {
                                handle_smart_light_toggle(state_clone.clone(), ip_clone)
                                    .await
                                    .unwrap();
                            });
                        }

                        checked=device.state == 1
                        class="sr-only peer"
                    />
                    <div class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                </label>
            </div>
        </li>
    }
}
