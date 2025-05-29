use {
    crate::{
        components::mish::{
            ipld_blob_page::get_ipld_blob_query, mish_state_page::get_mish_state_query,
        },
        integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on},
    },
    cid::Cid,
    ipld_core::codec::Codec,
    rhai::Dynamic,
    serde::{Deserialize, Serialize},
    serde_ipld_dagjson::codec::DagJsonCodec,
    std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}},
    tokio::{
        sync::mpsc::{UnboundedReceiver, UnboundedSender},
        time::{Duration, Instant},
    },
    tokio_cron_scheduler::{Job, JobScheduler},
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
    pool: &sqlx::PgPool,
    mut mish_state_modification_bus_receiver: UnboundedReceiver<MishStateModification>,
) {
    let mut lookup = HashMap::new();
    let mut job_scheduler = JobScheduler::new().await.unwrap();

    while let Some(mish_state_modification) = mish_state_modification_bus_receiver.recv().await {
        log::info!("Mish state modification: {:?}", mish_state_modification);
        match mish_state_modification {
            MishStateModification::CreateOrUpdate { name, state } => match name.as_str() {
                "run" => {
                    let result = serde_json::from_value::<HashMap<String, InstallItem>>(state);
                    match result {
                        Ok(items) => {
                            lookup.clear();
                            job_scheduler.shutdown().await.unwrap();
                            job_scheduler = JobScheduler::new().await.unwrap();
                            job_scheduler.start().await.unwrap();
                            for (name, item) in items {
                                log::info!("Installing {name}");
                                match item.clone() {
                                    InstallItem::MishStateAtMostOnceRhai {
                                        query_name,
                                        rhai,
                                        run_on_startup,
                                        ..
                                    } => {
                                        lookup.insert(query_name.clone(), item.clone());
                                        if run_on_startup {
                                            let state = get_mish_state_query(pool, &query_name)
                                                .await
                                                .unwrap();
                                            if let Some(state) = state {
                                                let scope = {
                                                    let state = serde_json::from_value(state.state);
                                                    match state {
                                                        Ok(state) => {
                                                            let mut scope = rhai::Scope::new();
                                                            scope.push_constant(
                                                                "name",
                                                                name.to_owned(),
                                                            );
                                                            scope.push_dynamic("state", state);
                                                            scope
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                                "Failed to parse fish tank state on: {:?}",
                                                                e
                                                            );
                                                            continue;
                                                        }
                                                    }
                                                };
                                                run_mish_state_at_most_once_rhai(
                                                    pool,
                                                    rhai.clone(),
                                                    scope,
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                    InstallItem::CronAtMostOnceRhai { cron_string, rhai } => {
                                        let pool = pool.clone();
                                        let rhai = rhai.clone();
                                        job_scheduler
                                            .add(
                                                Job::new_async(
                                                    cron_string.as_ref(),
                                                    move |_uuid, mut _l| {
                                                        let pool = pool.clone();
                                                        let rhai = rhai.clone();
                                                        Box::pin(async move {
                                                            let scope = rhai::Scope::new();
                                                            // scope.push_constant(
                                                            //     "name",
                                                            //     name.to_owned(),
                                                            // );
                                                            // scope.push_dynamic("state", state);
                                                            run_mish_state_at_most_once_rhai(
                                                                &pool,
                                                                rhai.clone(),
                                                                scope,
                                                            )
                                                            .await;
                                                        })
                                                    },
                                                )
                                                .unwrap(),
                                            )
                                            .await
                                            .unwrap();
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
                                let scope = {
                                    let state = serde_json::from_value(state);
                                    match state {
                                        Ok(state) => {
                                            let mut scope = rhai::Scope::new();
                                            scope.push_constant("name", name.to_owned());
                                            scope.push_dynamic("state", state);
                                            scope
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "Failed to parse fish tank state on: {:?}",
                                                e
                                            );
                                            continue;
                                        }
                                    }
                                };
                                run_mish_state_at_most_once_rhai(pool, rhai, scope).await;
                            }
                            InstallItem::CronAtMostOnceRhai { .. } => {
                                // TODO avoid panic
                                panic!("lookups should only be used for MishState types")
                            }
                        }
                    }
                }
            },
            MishStateModification::Delete { name: _ } => {}
        }
    }
    // TODO listen to events first and then poll the current state in-case something was missed
    // Specifically, I think we should just poll the current `run` but NOT the other queries. These should be `run` hooks instead.
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum InstallItem {
    MishStateAtMostOnceRhai {
        query_name: String,
        rhai: serde_json::Value,
        #[serde(default)]
        run_on_startup: bool,
    },
    CronAtMostOnceRhai {
        cron_string: String,
        rhai: serde_json::Value,
    },
}

async fn run_mish_state_at_most_once_rhai(
    pool: &sqlx::PgPool,
    rhai: serde_json::Value,
    scope: rhai::Scope<'static>,
) {
    // TODO refactor this and the AST compilation step to happen in the "run" handler
    let rhai_string = serde_json::from_value::<String>(rhai.clone());
    let rhai_cid =
        <DagJsonCodec as Codec<Cid>>::decode_from_slice(&serde_json::to_vec(&rhai).unwrap());
    let rhai = match (rhai_string, rhai_cid) {
        (Ok(rhai_string), Ok(rhai_cid)) => {
            panic!("Both String and Cid should not be parsable at the same time: {rhai_string} and {rhai_cid}");
        }
        (Ok(rhai_string), Err(_)) => rhai_string,
        (Err(_), Ok(rhai_cid)) => {
            if let Some(blob) = get_ipld_blob_query(pool, &rhai_cid).await.unwrap() {
                match String::from_utf8(blob) {
                    Ok(rhai_string) => rhai_string,
                    Err(e) => {
                        log::error!("Failed to parse fish tank state string on: {e}");
                        return;
                    }
                }
            } else {
                log::error!("Failed to get fish tank state on: {rhai_cid}");
                return;
            }
        }
        (Err(e1), Err(e2)) => {
            log::error!("Failed to parse fish tank state on: {e1} AND {e2}");
            return;
        }
    };
    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let mut scope = scope;
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
            .register_fn("unix_timestamp", || {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            })
            .register_fn("tplink_turn_plug_on", |ip: String| {
                tokio::task::spawn(async move {
                    tplink_turn_plug_on(&ip).await;
                });
            })
            .register_fn("tplink_turn_plug_off", |ip: String| {
                tokio::task::spawn(async move {
                    tplink_turn_plug_off(&ip).await;
                });
            })
            .run_with_scope(&mut scope, &rhai);
        if let Err(e) = result {
            log::error!("Failed to run fish tank script: {:?}", e);
        }
    });
}
