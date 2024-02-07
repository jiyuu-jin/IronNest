use leptos::*;

#[component]
pub fn Modal(toggle_modal: WriteSignal<bool>, modal: ReadSignal<bool>) -> impl IntoView {
    view! {
        <div class="relative z-10" aria-labelledby="modal-title" role="dialog" aria-modal="true">
            <div
                class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity"
                on:click=move |_| {
                    println!("clicked!");
                    spawn_local(async move {
                        toggle_modal.set(!modal.get());
                    })
                }
            >
            </div>
            <div class="fixed inset-0 z-10 w-screen overflow-y-auto">
                <div class="flex min-h-full items-end justify-center p-4 text-center sm:items-center sm:p-0">
                    <div class="relative transform overflow-hidden rounded-lg bg-white px-4 pb-4 pt-5 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-sm sm:p-6">
                        <div>
                            <div class="mt-3 text-center sm:mt-5">
                                <h3
                                    class="text-base font-semibold leading-6 text-gray-900"
                                    id="modal-title"
                                >
                                    "Device Settings"
                                </h3>
                                <div class="mt-2">
                                    <p class="text-sm text-gray-500"></p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
