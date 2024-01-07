use crate::integrations::iron_nest::types::DeviceType;

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

#[component]
pub fn DeviceListItem(device: Device) -> impl IntoView {
    match device.device_type {
        DeviceType::SmartPlug => view! { <SmartPlugItem device=device/> },
        DeviceType::SmartLight => view! { <SmartLightItem device=device/> },
        DeviceType::RingDoorbell => view! { <RingDoorbellItem device=device/> },
    }
}

#[server(HandleSmartPlugToggle)]
pub async fn handle_smart_plug_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on};
    if state {
        tplink_turn_plug_off(&ip).await;
    } else {
        tplink_turn_plug_on(&ip).await;
    }
    Ok(())
}

#[component]
pub fn SmartPlugItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let (signal, set_signal) = create_signal(device.power_state == 1);

    view! {
        <li class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow">
            <div class="flex w-full items-center justify-between space-x-6 p-6">
            <div class="flex-1 truncate">
                <div class="flex items-center space-x-3">
                <h3 class="truncate text-sm font-medium text-gray-900">{&device.name}</h3>
                <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">{format!("{}", &device.device_type)}</span>
                </div>
                <p class="mt-1 truncate text-sm text-gray-500">{&device.ip}</p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer ml-2 mt-2">
                    <input
                        type="checkbox"
                        value=""
                        on:click=move |_| {
                            let ip_clone = ip.clone();
                            println!("clicked!");
                            spawn_local(async move {
                                handle_smart_plug_toggle(signal.get(), ip_clone)
                                    .await
                                    .unwrap();
                                set_signal.set(!signal.get());
                            });
                        }
                        checked=signal
                        class="sr-only peer"
                    />
                    <div class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                </label>
            </div>
        </li>
    }
}

#[component]
pub fn RingDoorbellItem(device: Device) -> impl IntoView {
    view! {
        <li class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow">
            <div class="flex w-full items-center justify-between space-x-6 p-6">
                <div class="flex-1 truncate">
                    <div class="flex items-center space-x-3">
                        <h3 class="truncate text-sm font-medium text-gray-900">{&device.name}</h3>
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">{format!("{}", &device.device_type)}</span>
                    </div>
                    <p class="mt-1 truncate text-sm text-gray-500">{format!("Battery: {}", &device.battery_percentage)}</p>
                </div>
            </div>
        </li>
    }
}

#[server(HandleSmartLightToggle)]
pub async fn handle_smart_light_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::tplink::tplink_turn_light_on_off;
    tplink_turn_light_on_off(&ip, if state { 0 } else { 1 }).await;
    Ok(())
}

#[component]
pub fn SmartLightItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let (signal, set_signal) = create_signal(device.power_state == 1);

    view! {
        <li class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow">
            <div class="flex w-full items-center justify-between space-x-6 p-6">
            <div class="flex-1 truncate">
                <div class="flex items-center space-x-3">
                <h3 class="truncate text-sm font-medium text-gray-900">{&device.name}</h3>
                <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">{format!("{}", &device.device_type)}</span>
                </div>
                <p class="mt-1 truncate text-sm text-gray-500">{&device.ip}</p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer ml-2 mt-2">
                    <input
                        type="checkbox"
                        value=""
                        on:click=move |_| {
                            let ip_clone = ip.clone();
                            println!("clicked!");
                            spawn_local(async move {
                                handle_smart_plug_toggle(signal.get(), ip_clone)
                                    .await
                                    .unwrap();
                                set_signal.set(!signal.get());
                            });
                        }
                        checked=signal
                        class="sr-only peer"
                    />
                    <div class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                </label>
            </div>
        </li>
    }
}
