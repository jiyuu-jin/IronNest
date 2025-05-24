use {
    crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on},
    tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

#[derive(Debug, Clone)]
pub enum MishStateModification {
    CreateOrUpdate {
        name: String,
        state: serde_json::Value,
    },
    Delete {
        name: String,
    },
}

pub fn create_mish_state_modification_bus() -> (
    UnboundedSender<MishStateModification>,
    UnboundedReceiver<MishStateModification>,
) {
    tokio::sync::mpsc::unbounded_channel()
}

pub async fn register_native_queries(
    mut mish_state_modification_bus_receiver: UnboundedReceiver<MishStateModification>,
) {
    while let Some(mish_state_modification) = mish_state_modification_bus_receiver.recv().await {
        log::info!("Mish state modification: {:?}", mish_state_modification);
        match mish_state_modification {
            MishStateModification::CreateOrUpdate { name, state } => match name.as_str() {
                "chris.fish_tank.filter.pump" => {
                    // TODO do via mish
                }
                "chris.fish_tank.light.white" => {
                    // TODO do via mish
                }
                "chris.fish_tank.light.blue" => {
                    let blue_on_ip = "10.0.0.198";
                    let state = serde_json::from_value::<bool>(state);
                    match state {
                        Ok(state) => {
                            if state {
                                tplink_turn_plug_on(blue_on_ip).await;
                            } else {
                                tplink_turn_plug_off(blue_on_ip).await;
                            }
                        }
                        Err(_) => {
                            log::error!("Failed to parse blue light state on: {:?}", state);
                        }
                    }
                }
                _ => {}
            },
            MishStateModification::Delete { name: _ } => {}
        }
    }
    // TODO listen to events first and then poll the current state in-case something was missed
}
