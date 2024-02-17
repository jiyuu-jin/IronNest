use {
    crate::integrations::iron_nest::types::{Device, DeviceType},
    leptos::*,
    log::debug,
};

#[component]
pub fn DeviceView(device_type: DeviceType) -> impl IntoView {
    match device_type {
        DeviceType::SmartPlug => view! { <div></div> },
        DeviceType::SmartLight => view! { <div></div> },
        DeviceType::RingDoorbell => view! { <div></div> },
        DeviceType::RokuTv => view! { <div></div> },
        DeviceType::Stoplight => view! { <div></div> },
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
                                                        {data.name}
                                                    </h3>
                                                    <div class="mt-2">
                                                        <DeviceView device_type=data.device_type/>
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
pub fn SmartLightView() -> impl IntoView {}
