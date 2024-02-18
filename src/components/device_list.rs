use {
    crate::{
        components::{checkbox::Checkbox, device_list_card::DeviceListCard, device_modal::Modal},
        integrations::iron_nest::types::{Device, DeviceType},
        server::{
            roku::handle_roku_tv_toggle,
            tplink::{handle_smart_light_toggle, handle_smart_plug_toggle},
        },
    },
    leptos::*,
    log::debug,
};

#[component]
pub fn DeviceList(devices: Resource<(), Result<Vec<Device>, ServerFnError>>) -> impl IntoView {
    let (modal, toggle_modal) = create_signal(false);
    let (current_device, set_current_device) = create_signal(None);

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
                                                    view! {
                                                        <DeviceListItem
                                                            device=device.clone()
                                                            on:click=move |_| {
                                                                let state = modal.get();
                                                                debug!("clicked device list item! {state}");
                                                                toggle_modal.set(true);
                                                                set_current_device.set(Some(device.clone()))
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
            {move || {
                modal
                    .get()
                    .then(|| view! { <Modal toggle_modal=toggle_modal device=current_device/> })
            }}

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

#[component]
pub fn SmartPlugItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let (signal, set_signal) = create_signal(device.power_state == 1);

    view! {
        <DeviceListCard device=device>
            <Checkbox value=signal.get() on_click=Box::new(move || {
                let ip = ip.clone();
                set_signal.set(!signal.get());
                spawn_local(async move {
                    handle_smart_plug_toggle(signal.get(), ip).await.unwrap();
                })
            })/>
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

#[component]
pub fn SmartLightItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let (signal, set_signal) = create_signal(device.power_state == 1);

    view! {
        <DeviceListCard device=device>
            <Checkbox value=signal.get()on_click=Box::new(move || {
                let ip = ip.clone();
                set_signal.set(!signal.get());
                spawn_local(async move {
                    handle_smart_light_toggle(signal.get(), ip).await.unwrap();
                });
            })/>
        </DeviceListCard>
    }
}

#[component]
pub fn RokuTvItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    let (signal, set_signal) = create_signal(device.power_state == 1);

    view! {
        <DeviceListCard device=device>
            <Checkbox value=signal.get() on_click=Box::new(move || {
                let ip = ip.clone();
                set_signal.set(!signal.get());
                spawn_local(async move {
                    handle_roku_tv_toggle(signal.get(), ip).await.unwrap();
                });
            })/>
        </DeviceListCard>
    }
}
