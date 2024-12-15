use {
    crate::{
        components::checkbox::Checkbox,
        integrations::iron_nest::types::{Device, DeviceType},
        server::{
            roku::handle_roku_tv_toggle,
            tplink::{
                handle_smart_light_toggle, handle_smart_plug_toggle,
                handle_smart_power_strip_toggle,
            },
        },
    },
    leptos::prelude::*,
};

#[component]
pub fn DeviceListPanel(
    devices: Resource<Result<Vec<Device>, ServerFnError>>,
    device_ids: Vec<i64>,
) -> impl IntoView {
    view! {
        <div class="col-span-3 h-[264px] panel grid grid-rows-2 gap-2 rounded-lg">
            <Suspense fallback=|| {
                view! { <p class="text-center">"Loading devices..."</p> }
            }>
                {move || {
                    devices
                        .get()
                        .map(|result| {
                            match result {
                                Ok(data) => {
                                    let filtered_devices: Vec<Device> = data
                                        .into_iter()
                                        .filter(|device| device_ids.contains(&device.id))
                                        .collect();
                                    view! {
                                        <div class="grid grid-cols-2 gap-2">
                                            {filtered_devices
                                                .into_iter()
                                                .map(|device| {
                                                    view! { <DeviceCard device=device.clone()/> }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    }
                                        .into_any()
                                }
                                Err(e) => {
                                    view! {
                                        <p class="text-center text-red-500">
                                            {format!("Error: {e}")}
                                        </p>
                                    }
                                        .into_any()
                                }
                            }
                        })
                }}

            </Suspense>
        </div>
    }
}

#[component]
pub fn DeviceCard(device: Device) -> impl IntoView {
    view! {
        <div class="bg-white text-black p-2 rounded-lg shadow-md flex flex-col items-center justify-between">
            <p class="text-sm font-medium text-nowrap w-full">{device.name.clone()}</p>
            <div class="mt-2">
                {match device.device_type {
                    DeviceType::KasaPlug => {
                        view! { <SmartPlugItem device=device.clone()/> }.into_any()
                    }
                    DeviceType::KasaLight => {
                        view! { <SmartLightItem device=device.clone()/> }.into_any()
                    }
                    DeviceType::KasaDimmer => {
                        view! { <SmartDimmerItem device=device.clone()/> }.into_any()
                    }
                    DeviceType::KasaPowerStrip => {
                        view! { <SmartPowerStripItem device=device.clone()/> }.into_any()
                    }
                    DeviceType::TuyaLight => {
                        view! { <SmartLightItem device=device.clone()/> }.into_any()
                    }
                    DeviceType::TuyaGrowLight => {
                        view! { <SmartLightItem device=device.clone()/> }.into_any()
                    }
                    DeviceType::RingDoorbell => view! { <RingDoorbellItem/> }.into_any(),
                    DeviceType::Stoplight => view! { <StoplightItem/> }.into_any(),
                    DeviceType::RokuTv => view! { <RokuTvItem device=device.clone()/> }.into_any(),
                }}

            </div>
        </div>
    }
}

#[component]
pub fn SmartPlugItem(device: Device) -> impl IntoView {
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
pub fn SmartDimmerItem(device: Device) -> impl IntoView {
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
pub fn SmartPowerStripItem(device: Device) -> impl IntoView {
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        let child_id = device.child_id.clone().unwrap();
        move |value| {
            let ip = ip.clone();
            let child_id = child_id.clone();
            let value = *value;
            async move {
                handle_smart_power_strip_toggle(value, ip, child_id)
                    .await
                    .unwrap();
            }
        }
    });

    view! { <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/> }
}

#[component]
pub fn RingDoorbellItem() -> impl IntoView {
    view! { <></> }
}

#[component]
pub fn StoplightItem() -> impl IntoView {
    view! { <></> }
}

#[component]
pub fn SmartLightItem(device: Device) -> impl IntoView {
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

    view! { <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/> }
}

#[component]
pub fn RokuTvItem(device: Device) -> impl IntoView {
    let toggle_action = Action::new({
        let ip = device.ip.clone();
        move |value| {
            let ip = ip.clone();
            let value = *value;
            async move {
                handle_roku_tv_toggle(value, ip).await.unwrap();
            }
        }
    });

    view! { <Checkbox value=device.power_state == 1 on_click=Some(toggle_action) on_click_fn=None/> }
}
