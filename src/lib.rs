pub mod components;
pub mod error_template;
pub mod integrations;
pub mod server;

#[cfg(feature = "ssr")]
pub mod app;
#[cfg(feature = "ssr")]
pub mod handlers;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::layout::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
