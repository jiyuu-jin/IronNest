use {
    crate::integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on},
    rhai::Dynamic,
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
    tokio::{
        sync::mpsc::{UnboundedReceiver, UnboundedSender},
        time::{Duration, Instant},
    },
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
    let mut lookup = HashMap::new();
    while let Some(mish_state_modification) = mish_state_modification_bus_receiver.recv().await {
        log::info!("Mish state modification: {:?}", mish_state_modification);
        match mish_state_modification {
            MishStateModification::CreateOrUpdate { name, state } => match name.as_str() {
                "run" => {
                    let result = serde_json::from_value::<Vec<InstallItem>>(state);
                    match result {
                        Ok(items) => {
                            lookup.clear();
                            for item in items {
                                match &item {
                                    InstallItem::MishStateAtMostOnceRhai { query_name, .. } => {
                                        lookup.insert(query_name.clone(), item);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse install items: {e}");
                        }
                    }
                }
                name => {
                    if let Some(item) = lookup.get(name).cloned() {
                        match item {
                            InstallItem::MishStateAtMostOnceRhai { rhai, .. } => {
                                tokio::task::spawn_blocking(move || {
                                    let state = serde_json::from_value(state);
                                    match state {
                                        Ok(state) => {
                                            let start = Instant::now();
                                            let mut scope = rhai::Scope::new();
                                            scope.push_dynamic("state", state);
                                            let result = rhai::Engine::new()
                                                .on_progress(move |_| {
                                                    if start.elapsed() > Duration::from_secs(10) {
                                                        // Return a dummy token just to force-terminate the script
                                                        Some(Dynamic::UNIT)
                                                    } else {
                                                        // Continue
                                                        None
                                                    }
                                                })
                                                .register_fn("tplink_turn_plug_on", |ip: String| {
                                                    tokio::task::spawn(async move {
                                                        tplink_turn_plug_on(&ip).await;
                                                    });
                                                })
                                                .register_fn(
                                                    "tplink_turn_plug_off",
                                                    |ip: String| {
                                                        tokio::task::spawn(async move {
                                                            tplink_turn_plug_off(&ip).await;
                                                        });
                                                    },
                                                )
                                                .run_with_scope(&mut scope, &rhai);
                                            if let Err(e) = result {
                                                log::error!(
                                                    "Failed to run fish tank script: {:?}",
                                                    e
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "Failed to parse fish tank state on: {:?}",
                                                e
                                            );
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            },
            MishStateModification::Delete { name: _ } => {}
        }
    }
    // TODO listen to events first and then poll the current state in-case something was missed
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum InstallItem {
    MishStateAtMostOnceRhai { query_name: String, rhai: String },
}
