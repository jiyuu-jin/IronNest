use {
    crate::{
        components::{device_list_card::DeviceListCard, modal::Modal},
        integrations::iron_nest::types::{Device, DeviceType},
    },
    leptos::*,
    log::debug,
};

#[component]
pub fn DeviceList(devices: Resource<(), Result<Vec<Device>, ServerFnError>>) -> impl IntoView {
    let (modal, toggle_modal) = create_signal(false);
    // let (current_device, set_current_device) = create_signal(None);

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
                            match data {
                                Ok(data) => {
                                    view! {
                                        <ul class="device-list space-y-2">
                                            {data
                                                .into_iter()
                                                .map(|device| {
                                                    let device_clone = device.clone();
                                                    view! {
                                                        <DeviceListItem
                                                            device=device.clone()
                                                            on:click=move |_| {
                                                                let state = modal.get();
                                                                debug!("clicked device list item! {state}");
                                                                toggle_modal.set(true);
                                                            }
                                                        />
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    }
                                        .into_view()
                                }
                                Err(e) => {
                                    view! {
                                        // set_current_device.set(Some(device_clone))

                                        <p>{format!("DeviceList error: {e}")}</p>
                                    }
                                        .into_view()
                                }
                            }
                        })
                }}

            </Suspense>
            {move || modal.get().then(|| view! { <Modal toggle_modal=toggle_modal/> })}
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
        <DeviceListCard device=device>
            <label
                class="relative inline-flex items-center cursor-pointer ml-2 mt-2"
                on:click:undelegated=|e| {
                    e.stop_propagation();
                }
            >
                <input
                    type="checkbox"
                    value=""
                    on:click:undelegated=move |_| {
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
        </DeviceListCard>
    }
}

#[component]
pub fn RingDoorbellItem(device: Device) -> impl IntoView {
    view! {
        <DeviceListCard device=device>
            <div></div>
        </DeviceListCard>
    }
}

#[component]
pub fn StoplightItem(device: Device) -> impl IntoView {
    view! {
        <DeviceListCard device=device>
            <div></div>
        </DeviceListCard>
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
        <DeviceListCard device=device>
            <label
                class="relative inline-flex items-center cursor-pointer ml-2 mt-2"
                on:click:undelegated=|e| {
                    e.stop_propagation();
                }
            >
                <input
                    type="checkbox"
                    value=""
                    on:click:undelegated=move |_| {
                        let ip_clone = ip.clone();
                        let signal = signal.get();
                        debug!("clicked checkbox!");
                        spawn_local(async move {
                            handle_smart_light_toggle(signal, ip_clone).await.unwrap();
                            set_signal.set(!signal);
                        });
                    }

                    checked=signal
                    class="sr-only peer"
                />
                <div class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
            </label>
        </DeviceListCard>
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
        <DeviceListCard device=device>
            <label
                class="relative inline-flex items-center cursor-pointer ml-2 mt-2"
                on:click:undelegated=|e| {
                    e.stop_propagation();
                }
            >
                <input
                    type="checkbox"
                    value=""
                    on:click:undelegated=move |_| {
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
        </DeviceListCard>
    }
}
