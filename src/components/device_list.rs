use {
    crate::{
        components::modal::Modal,
        integrations::iron_nest::types::{Device, DeviceType},
    },
    leptos::*,
};

#[component]
pub fn DeviceList(devices: Resource<(), Result<Vec<Device>, ServerFnError>>) -> impl IntoView {
    let (modal, toggle_modal) = create_signal(false);

    view! {
        <div>
            <h2 class="text-lg">"Devices"</h2>
            <hr class="mb-2"/>
            <Suspense fallback=|| {
                view! { <p>"Loading devices..."</p> }
            }>
                {move || {
                    devices
                        .get()
                        .map(|data| {
                            data.map(|data| {
                                view! {
                                    <ul class="device-list space-y-2">
                                        {data
                                            .into_iter()
                                            .map(|device| {
                                                view! {
                                                    <DeviceListItem
                                                        device=device
                                                        on:click=move |_| {
                                                            println!("clicked!");
                                                            spawn_local(async move {
                                                                toggle_modal.set(!modal.get());
                                                            });
                                                        }
                                                    />
                                                }
                                            })
                                            .collect::<Vec<_>>()}
                                    </ul>
                                }
                            })
                        })
                }}
                >>>>>>> Stashed changes
            </Suspense>
            {move || modal.get().then(|| view! { <Modal modal=modal toggle_modal=toggle_modal/> })}

        </div>
    }
}

#[component]
pub fn DeviceListItem(device: Device) -> impl IntoView {
    match device.device_type {
        DeviceType::SmartPlug => view! { <SmartPlugItem device=device/> },
        DeviceType::SmartLight => view! { <SmartLightItem device=device/> },
        DeviceType::RingDoorbell => view! { <RingDoorbellItem device=device/> },
        DeviceType::Stoplight => view! { <StoplightItem device=device/> },
        DeviceType::RokuTv => view! { <RokuTvItem device=device/> },
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
        <li
            style="min-height: 100px"
            class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow"
        >
            <div class="flex w-full items-center justify-between space-x-6 p-6">
                <div class="flex-1 truncate">
                    <div class="flex items-center space-x-3">
                        <h3 class="truncate text-sm font-medium text-gray-900">{&device.name}</h3>
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                                stroke="currentColor"
                                class="w-6 h-6"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="m3.75 13.5 10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75Z"
                                ></path>
                            </svg>
                        </span>
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
                                handle_smart_plug_toggle(signal.get(), ip_clone).await.unwrap();
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
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                                stroke="currentColor"
                                class="w-6 h-6"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M14.857 17.082a23.848 23.848 0 0 0 5.454-1.31A8.967 8.967 0 0 1 18 9.75V9A6 6 0 0 0 6 9v.75a8.967 8.967 0 0 1-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 0 1-5.714 0m5.714 0a3 3 0 1 1-5.714 0"
                                ></path>
                            </svg>
                        </span>
                    </div>
                    <p class="mt-1 truncate text-sm text-gray-500">
                        {format!("Battery: {}", &device.battery_percentage)}
                    </p>
                </div>
            </div>
        </li>
    }
}

#[component]
pub fn StoplightItem(device: Device) -> impl IntoView {
    view! {
        <li
            style="min-height: 100px"
            class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow"
        >
            <div class="flex w-full items-center justify-between space-x-6 p-6">
                <div class="flex-1 truncate">
                    <div class="flex items-center space-x-3">
                        <h3 class="truncate text-sm font-medium text-gray-900">{&device.name}</h3>
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                            <svg
                                width="24"
                                height="24"
                                viewBox="0 0 24 24"
                                fill="none"
                                xmlns="http://www.w3.org/2000/svg"
                            >
                                <path d="M15 9H9V15H15V9Z" fill="currentColor"></path>
                                <path
                                    fill-rule="evenodd"
                                    clip-rule="evenodd"
                                    d="M23 12C23 18.0751 18.0751 23 12 23C5.92487 23 1 18.0751 1 12C1 5.92487 5.92487 1 12 1C18.0751 1 23 5.92487 23 12ZM21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12Z"
                                    fill="currentColor"
                                ></path>
                            </svg>
                        </span>
                    </div>
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
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                                stroke="currentColor"
                                class="w-6 h-6"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M12 18v-5.25m0 0a6.01 6.01 0 0 0 1.5-.189m-1.5.189a6.01 6.01 0 0 1-1.5-.189m3.75 7.478a12.06 12.06 0 0 1-4.5 0m3.75 2.383a14.406 14.406 0 0 1-3 0M14.25 18v-.192c0-.983.658-1.823 1.508-2.316a7.5 7.5 0 1 0-7.517 0c.85.493 1.509 1.333 1.509 2.316V18"
                                ></path>
                            </svg>
                        </span>
                    </div>
                    <p class="mt-1 truncate text-sm text-gray-500">{&device.ip}</p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer ml-2 mt-2">
                    <input
                        type="checkbox"
                        value=""
                        on:click=move |_| {
                            let ip_clone = ip.clone();
                            spawn_local(async move {
                                handle_smart_light_toggle(signal.get(), ip_clone).await.unwrap();
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

#[server(HandleRokuTvToggle)]
pub async fn handle_roku_tv_toggle(state: bool, ip: String) -> Result<(), ServerFnError> {
    use crate::integrations::roku::roku_send_keypress;
    if state {
        roku_send_keypress(&ip, "PowerOff").await;
    } else {
        roku_send_keypress(&ip, "PowerOn").await;
    }
    Ok(())
}

#[component]
pub fn RokuTvItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let (signal, set_signal) = create_signal(device.power_state == 1);

    view! {
        <li class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow">
            <div class="flex w-full items-center justify-between space-x-6 p-6">
                <div class="flex-1 truncate">
                    <div class="flex items-center space-x-3">
                        <h3 class="truncate text-sm font-medium text-gray-900">{&device.name}</h3>
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke-width="1.5"
                                stroke="currentColor"
                                class="w-6 h-6"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    d="M6 20.25h12m-7.5-3v3m3-3v3m-10.125-3h17.25c.621 0 1.125-.504 1.125-1.125V4.875c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125Z"
                                ></path>
                            </svg>
                        </span>
                    </div>
                    <p class="mt-1 truncate text-sm text-gray-500">{&device.ip}</p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer ml-2 mt-2">
                    <input
                        type="checkbox"
                        value=""
                        on:click=move |_| {
                            let ip_clone = ip.clone();
                            spawn_local(async move {
                                handle_roku_tv_toggle(signal.get(), ip_clone).await.unwrap();
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
