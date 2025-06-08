#![recursion_limit = "512"]

pub mod components;
pub mod error_template;

#[cfg(feature = "ssr")]
pub mod handlers;
pub mod integrations;
#[cfg(feature = "ssr")]
pub mod mish_api;
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::layout::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

pub mod ipld_codecs {
    use {ipld_core::codec::Codec, serde_ipld_dagjson::codec::DagJsonCodec};

    // https://github.com/multiformats/multicodec/blob/3bc7f4c20afe28e10d9d539e2a565578de6dd71c/table.csv#L41
    pub const RAW: u64 = 0x55;
    pub const DAG_JSON: u64 = <DagJsonCodec as Codec<serde_json::Value>>::CODE;
}
