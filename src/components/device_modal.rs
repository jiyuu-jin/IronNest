use {
    super::checkbox::Checkbox,
    crate::{
        components::{color_picker::ColorPicker, slider::Slider},
        integrations::iron_nest::types::{Device, DeviceType},
        server::tplink::{
            handle_smart_dimmer_brightness, handle_smart_light_brightness, handle_smart_light_hsl,
            handle_smart_light_toggle, handle_smart_plug_toggle,
        },
    },
    leptos::{prelude::*, task::spawn_local},
};

#[component]
pub fn DeviceView(device: Device) -> impl IntoView {
    match device.device_type {
        DeviceType::KasaPlug => view! { <SmartPlugView device=device/> }.into_any(),
        DeviceType::KasaLight => view! { <SmartLightView device=device/> }.into_any(),
        DeviceType::KasaDimmer => view! { <SmartDimmerView device=device/> }.into_any(),
        DeviceType::KasaPowerStrip => view! { <SmartPowerStripView device=device/> }.into_any(),
        DeviceType::RingDoorbell => view! { <RingDoorbellView device=device/> }.into_any(),
        DeviceType::RokuTv => view! { <RokuTvView device=device/> }.into_any(),
        DeviceType::Stoplight => view! { <StoplightView device=device/> }.into_any(),
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
                    leptos::logging::log!("clicked!");
                    toggle_modal.set(false);
                }
            >

                <div class="flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0">
                    <div
                        class="relative transform overflow-hidden rounded-lg bg-white px-4 pb-4 pt-5 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-sm sm:p-6"
                        on:click=|e| e.stop_propagation()
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
                                                .into_any()
                                        }
                                        None => view! { <div></div> }.into_any(),
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
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        move |value| {
            let ip = ip.clone();
            let value = *value;
            async move {
                handle_smart_light_toggle(value, ip).await.unwrap();
            }
        }
    });

    view! {
        <div class="flex flex-col">
            <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/>
            <Slider on_change=Box::new({
                let ip = device.ip.clone();
                move |value| {
                    let ip = ip.clone();
                    spawn_local(async move {
                        handle_smart_light_brightness(value, ip).await.unwrap();
                    });
                }
            })/>

            <ColorPicker
                label="Color".to_string()
                default_value="#e66465".to_string()
                on_change=Box::new({
                    let ip = device.ip.clone();
                    move |value| {
                        leptos::logging::log!("color_picker on change outer: {value}");
                        let ip = ip.clone();
                        spawn_local(async move {
                            leptos::logging::log!("parsing: {value}");
                            handle_smart_light_hsl(ip, value).await.unwrap();
                        });
                    }
                })
            />

        </div>
    }
}

#[component]
pub fn SmartPlugView(device: Device) -> impl IntoView {
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        move |value| {
            let ip = ip.clone();
            let value = *value;
            async move {
                handle_smart_plug_toggle(value, ip).await.unwrap();
            }
        }
    });

    view! { <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/> }
}

#[component]
pub fn SmartDimmerView(device: Device) -> impl IntoView {
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        move |value| {
            let ip = ip.clone();
            let value = *value;
            async move {
                handle_smart_plug_toggle(value, ip).await.unwrap();
            }
        }
    });

    view! {
        <div class="flex flex-col">
            <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/>
            <Slider on_change=Box::new({
                let ip = device.ip.clone();
                move |value| {
                    let ip = ip.clone();
                    spawn_local(async move {
                        handle_smart_dimmer_brightness(value, ip).await.unwrap();
                    });
                }
            })/>
        </div>
    }
}

#[component]
pub fn SmartPowerStripView(device: Device) -> impl IntoView {
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        move |value| {
            let ip = ip.clone();
            let value = *value;
            async move {
                handle_smart_plug_toggle(value, ip).await.unwrap();
            }
        }
    });

    view! { <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/> }
}

#[component]
pub fn RingDoorbellView(device: Device) -> impl IntoView {
    view! { <div>"Power State: " {device.battery_percentage}</div> }
}

#[component]
pub fn RokuTvView(device: Device) -> impl IntoView {
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        move |value| {
            let ip = ip.clone();
            let value = *value;
            async move {
                handle_smart_plug_toggle(value, ip).await.unwrap();
            }
        }
    });

    view! { <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/> }
}

#[component]
pub fn StoplightView(device: Device) -> impl IntoView {
    view! { <div>"Power State: " {device.battery_percentage}</div> }
}
