use leptos::{logging::log, prelude::*};

#[component]
pub fn Checkbox(
    value: bool,
    on_click: Option<Action<bool, ()>>,
    on_click_fn: Option<Box<dyn Fn()>>,
) -> impl IntoView {
    let (signal, set_signal) = signal(value);
    view! {
        <label
            class="relative inline-flex items-center cursor-pointer ml-2 mt-2"
            on:click=move |e| {
                log!("clicked propagated!");
                e.stop_propagation();
            }
        >

            <input
                type="checkbox"
                checked=move || signal.get()
                on:input:target=move |ev| {
                    log!("clicked!");
                    set_signal.set(!signal.get());
                    if let Some(a) = &on_click {
                        a.dispatch(event_target_checked(&ev));
                    }
                    if let Some(f) = &on_click_fn {
                        f();
                    }
                }

                class="sr-only peer"
            />
            <div class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
        </label>
    }
}
