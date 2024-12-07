use {
    super::dashboard_page::DashboardValues,
    crate::components::pages::dashboard_page::get_dashboard_values, leptos::prelude::*,
};

#[component]
pub fn WebSocketPage() -> impl IntoView {
    let ring_values = Resource::new(|| (), |_| get_dashboard_values());

    view! {
        <h1>"WebSocket"</h1>
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || {
                ring_values
                    .get()
                    .map(|ring_values| {
                        match ring_values {
                            Ok(_ring_values) => view! { <div></div> }.into_any(),
                            Err(e) => {
                                view! { <p>{format!("WebSocketPage error: {e}")}</p> }.into_any()
                            }
                        }
                    })
            }}

        </Suspense>
    }
}

#[component]
fn WebSocketComponent(_ring_values: DashboardValues) -> impl IntoView {
    // let UseWebsocketReturn {
    //     ready_state,
    //     message,
    //     message_bytes,
    //     send,
    //     send_bytes,
    //     open,
    //     close,
    //     ..
    // } = use_websocket(&ring_values.ws_url);

    // let send_message = move |_| {
    //     send("Hello, world!");
    // };

    // let send_byte_message = move |_| {
    //     send_bytes(b"Hello, world!\r\n".to_vec());
    // };

    // create_effect(move |_| {
    //     if ready_state.get() == ConnectionReadyState::Open {
    //         let send_bytes = send_bytes.clone();
    //         let pc = RtcPeerConnection::new().unwrap();
    //         let create_offer_callback = Closure::wrap(Box::new(move |offer: JsValue| {
    //             let sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))
    //                 .unwrap()
    //                 .as_string()
    //                 .unwrap();
    //             send_bytes(
    //                 serde_json::to_vec(&json!({
    //                     "method": "live_view",
    //                     "dialog_id": "333333",
    //                     "body": {
    //                         "doorbot_id": "",
    //                         "stream_options": { "audio_enabled": true, "video_enabled": true },
    //                         "sdp": sdp,
    //                     }
    //                 }))
    //                 .unwrap(),
    //             );
    //         }) as Box<dyn FnMut(JsValue)>);
    //         let _ = pc.create_offer().then(&create_offer_callback);
    //         create_offer_callback.forget();
    //     }
    // });
    // let status = move || ready_state.get().to_string();

    // let connected = move || ready_state.get() == ConnectionReadyState::Open;

    // let open_connection = move |_| {
    //     open();
    // };

    // let close_connection = move |_| {
    //     close();
    // };

    view! { <div></div> }
}
