use {
    crate::{
        components::{checkbox::Checkbox, device_list_card::DeviceListCard, device_modal::Modal},
        integrations::iron_nest::types::{Device, DeviceType},
        server::{
            roku::handle_roku_tv_toggle,
            tplink::{handle_smart_light_toggle, handle_smart_plug_toggle},
        },
    },
    leptos::{html::Div, *},
    leptos_use::{
        core::Position, use_draggable_with_options, UseDraggableOptions, UseDraggableReturn,
    },
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
    let el = create_node_ref::<Div>();

    // let UseDraggableReturn { x, y, style, .. } = use_draggable_with_options(
    //     el,
    //     UseDraggableOptions::default().initial_value(Position { x: 40.0, y: 40.0 }),
    // );

    match device.device_type {
        DeviceType::SmartPlug => view! {
            <div>
                <SmartPlugItem device=device/>
            </div>
        },
        DeviceType::SmartLight => view! {
            <div>
                <SmartLightItem device=device/>
            </div>
        },
        DeviceType::RingDoorbell => view! {
            <div>
                <RingDoorbellItem device=device/>
            </div>
        },
        DeviceType::Stoplight => view! {
            <div>
                <StoplightItem device=device/>
            </div>
        },
        DeviceType::RokuTv => view! {
            <div>
                <RokuTvItem device=device/>
            </div>
        },
    }
}

#[component]
pub fn SmartPlugItem(device: Device) -> impl IntoView {
    let device_clone = device.clone();
    let ip = device_clone.ip.clone();

    view! {
        <DeviceListCard device=device.clone()>
            <Checkbox
                value=device.power_state == 1
                on_click=Box::new(move |value| {
                    let ip = ip.clone();
                    spawn_local(async move {
                        handle_smart_plug_toggle(value, ip).await.unwrap();
                    })
                })
            />
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
    view! {
        <DeviceListCard device=device.clone()>
            <Checkbox
                value=device.power_state == 1
                on_click=Box::new(move |value| {
                    let ip = ip.clone();
                    spawn_local(async move {
                        handle_smart_light_toggle(value, ip).await.unwrap();
                    });
                })
            />
        </DeviceListCard>
    }
}

#[component]
pub fn RokuTvItem(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    view! {
        <DeviceListCard device=device.clone()>
            <Checkbox
                value=device.power_state == 1
                on_click=Box::new(move |value| {
                    let ip = ip.clone();
                    spawn_local(async move {
                        handle_roku_tv_toggle(value, ip).await.unwrap();
                    });
                })
            />
        </DeviceListCard>
    }
}
