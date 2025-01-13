pub mod components;
pub mod error_template;

#[cfg(feature = "ssr")]
pub mod handlers;
pub mod integrations;
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::layout::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
