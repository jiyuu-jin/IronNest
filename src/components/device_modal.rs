use {
    super::checkbox::Checkbox,
    crate::{
        components::slider::Slider,
        integrations::iron_nest::types::{Device, DeviceType},
        server::tplink::{
            handle_smart_light_brightness, handle_smart_light_toggle, handle_smart_plug_toggle,
        },
    },
    leptos::*,
    log::debug,
};

#[component]
pub fn DeviceView(device: Device) -> impl IntoView {
    match device.device_type {
        DeviceType::SmartPlug => view! { <SmartPlugView device=device/> },
        DeviceType::SmartLight => view! { <SmartLightView device=device/> },
        DeviceType::RingDoorbell => view! { <RingDoorbellView device=device/> },
        DeviceType::RokuTv => view! { <RokuTvView device=device/> },
        DeviceType::Stoplight => view! { <StoplightView device=device/> },
    }
}

#[component]
pub fn Modal(toggle_modal: WriteSignal<bool>, device: ReadSignal<Option<Device>>) -> impl IntoView {
    view! {
        <div class="relative z-10" aria-labelledby="modal-title" role="dialog" aria-modal="true">
            <div class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity"></div>
            <div
                class="fixed inset-0 z-10 w-screen overflow-y-auto"
                on:click=move |_| {
                    debug!("clicked!");
                    toggle_modal.set(false);
                }
            >

                <div class="flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0">
                    <div
                        class="relative transform overflow-hidden rounded-lg bg-white px-4 pb-4 pt-5 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-sm sm:p-6"
                        on:click:undelegated=|e| e.stop_propagation()
                    >
                        <div>
                            <div class="mt-3 text-center sm:mt-5">
                                {move || {
                                    match device.get() {
                                        Some(data) => {
                                            view! {
                                                <div>
                                                    <h3
                                                        class="text-base font-semibold leading-6 text-gray-900"
                                                        id="modal-title"
                                                    >
                                                        {data.name.to_owned()}
                                                    </h3>
                                                    <div class="mt-2">
                                                        <DeviceView device=data/>
                                                    </div>
                                                </div>
                                            }
                                        }
                                        None => {
                                            view! { <div></div> }
                                        }
                                    }
                                }}

                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn SmartLightView(device: Device) -> impl IntoView {
    let ip = device.ip.clone();

    view! {
        <div class="flex flex-col">
            <Checkbox
                value=device.power_state == 1
                on_click=Box::new(move |value| {
                    let ip_clone = device.ip.clone();
                    spawn_local(async move {
                        handle_smart_light_toggle(value, ip_clone).await.unwrap();
                    });
                })
            />

            <Slider
                on_change=Box::new(move |value| {
                    let ip_clone = ip.clone();
                    spawn_local(async move {
                        handle_smart_light_brightness(value, ip_clone).await.unwrap();
                    });
            })/>
        </div>
    }
}

#[component]
pub fn SmartPlugView(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    view! {
        <Checkbox
            value=device.power_state == 1
            on_click=Box::new(move |value| {
                let ip = ip.clone();
                spawn_local(async move {
                    handle_smart_plug_toggle(value, ip).await.unwrap();
                })
            })
        />
    }
}

#[component]
pub fn RingDoorbellView(device: Device) -> impl IntoView {
    view! { <div>"Power State: " {device.battery_percentage}</div> }
}

#[component]
pub fn RokuTvView(device: Device) -> impl IntoView {
    let ip = device.ip.to_string();
    view! {
        <Checkbox
            value=device.power_state == 1
            on_click=Box::new(move |value| {
                let ip = ip.clone();
                spawn_local(async move {
                    handle_smart_plug_toggle(value, ip).await.unwrap();
                })
            })
        />
    }
}

#[component]
pub fn StoplightView(device: Device) -> impl IntoView {
    view! { <div>"Power State: " {device.battery_percentage}</div> }
}
