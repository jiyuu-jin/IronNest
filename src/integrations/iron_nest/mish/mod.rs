use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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
        println!("Mish state modification: {:?}", mish_state_modification);
    }
}
