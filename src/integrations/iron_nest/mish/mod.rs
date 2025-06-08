use {
    crate::{
        components::mish::{
            ipld_blob_page::get_ipld_blob_query, mish_state_page::get_mish_state_query,
        },
        integrations::tplink::{tplink_turn_plug_off, tplink_turn_plug_on},
        mish_api::{update_mish_state, UpdateMishStateBody},
    },
    cid::Cid,
    ipld_core::codec::Codec,
    rhai::Dynamic,
    serde::{Deserialize, Serialize},
    serde_ipld_dagjson::codec::DagJsonCodec,
    std::{
        collections::HashMap,
        str::FromStr,
        time::{SystemTime, UNIX_EPOCH},
    },
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
    mish_state_modification_bus_sender: UnboundedSender<MishStateModification>,
) {
    let mut lookup = HashMap::new();
    let mut job_scheduler = JobScheduler::new().await.unwrap();

    let state = get_mish_state_query(pool, "run").await.unwrap();
    if let Some(state) = state {
        do_install(
            pool,
            mish_state_modification_bus_sender.clone(),
            &mut lookup,
            &mut job_scheduler,
            state.state.clone(),
        )
        .await;
    }

    while let Some(mish_state_modification) = mish_state_modification_bus_receiver.recv().await {
        log::info!("Mish state modification: {:?}", mish_state_modification);
        match mish_state_modification {
            MishStateModification::CreateOrUpdate { name, state } => match name.as_str() {
                "run" => {
                    do_install(
                        pool,
                        mish_state_modification_bus_sender.clone(),
                        &mut lookup,
                        &mut job_scheduler,
                        state,
                    )
                    .await;
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
                                run_mish_state_at_most_once_rhai(
                                    pool.clone(),
                                    mish_state_modification_bus_sender.clone(),
                                    rhai,
                                    scope,
                                )
                                .await;
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

async fn do_install(
    pool: &sqlx::PgPool,
    mish_state_modification_bus_sender: UnboundedSender<MishStateModification>,
    lookup: &mut HashMap<String, InstallItem>,
    job_scheduler: &mut JobScheduler,
    state: serde_json::Value,
) {
    let result = serde_json::from_value::<HashMap<String, InstallItem>>(state);
    match result {
        Ok(items) => {
            lookup.clear();
            job_scheduler.shutdown().await.unwrap();
            *job_scheduler = JobScheduler::new().await.unwrap();
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
                            let state = get_mish_state_query(pool, &query_name).await.unwrap();
                            if let Some(state) = state {
                                let scope = {
                                    let state = serde_json::from_value(state.state);
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
                                run_mish_state_at_most_once_rhai(
                                    pool.clone(),
                                    mish_state_modification_bus_sender.clone(),
                                    rhai.clone(),
                                    scope,
                                )
                                .await;
                            }
                        }
                    }
                    InstallItem::CronAtMostOnceRhai { cron_string, rhai } => {
                        let pool = pool.clone();
                        let mish_state_modification_bus_sender =
                            mish_state_modification_bus_sender.clone();
                        let rhai = rhai.clone();
                        job_scheduler
                            .add(
                                Job::new_async(cron_string.as_ref(), move |_uuid, mut _l| {
                                    let pool = pool.clone();
                                    let mish_state_modification_bus_sender =
                                        mish_state_modification_bus_sender.clone();
                                    let rhai = rhai.clone();
                                    Box::pin(async move {
                                        let scope = rhai::Scope::new();
                                        run_mish_state_at_most_once_rhai(
                                            pool,
                                            mish_state_modification_bus_sender,
                                            rhai,
                                            scope,
                                        )
                                        .await;
                                    })
                                })
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

async fn run_mish_state_at_most_once_rhai(
    pool: sqlx::PgPool,
    mish_state_modification_bus_sender: UnboundedSender<MishStateModification>,
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
            if let Some(blob) = get_ipld_blob_query(&pool, &rhai_cid).await.unwrap() {
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
                    .as_secs() as i64 // conversion to i64 needed or else modulos in the script will fail
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
            .register_fn(
                "update_mish_state",
                move |name: String, path: String, content: Dynamic| {
                    let pool = pool.clone();
                    let mish_state_modification_bus_sender =
                        mish_state_modification_bus_sender.clone();
                    let content = serde_json::to_value(&content).unwrap();
                    tokio::task::spawn(async move {
                        if let Err(e) = update_mish_state(
                            &pool,
                            &mish_state_modification_bus_sender,
                            UpdateMishStateBody {
                                mish_state_name: name,
                                path,
                                content,
                            },
                        )
                        .await
                        {
                            log::error!("Failed to update mish state: {e}");
                        }
                    });
                },
            )
            .register_fn(
                "is_now_between",
                |timezone: String, start: String, up_to: String| {
                    is_now_between(&timezone, &start, &up_to, chrono::Utc::now())
                },
            )
            .run_with_scope(&mut scope, &rhai);
        if let Err(e) = result {
            log::error!("Failed to run fish tank script: {:?}", e);
        }
    });
}

fn is_now_between(
    timezone: &str,
    start: &str,
    up_to: &str,
    current_time: chrono::DateTime<chrono::Utc>,
) -> bool {
    // Parse timezone
    let tz = match chrono_tz::Tz::from_str(timezone) {
        Ok(tz) => tz,
        Err(_) => {
            log::error!("Invalid timezone: {}", timezone);
            return false;
        }
    };

    // Parse time strings
    let start_time = match chrono::NaiveTime::parse_from_str(start, "%H:%M:%S") {
        Ok(time) => time,
        Err(_) => {
            log::error!("Invalid start time format: {}", start);
            return false;
        }
    };

    let up_to_time = match chrono::NaiveTime::parse_from_str(up_to, "%H:%M:%S") {
        Ok(time) => time,
        Err(_) => {
            log::error!("Invalid up_to time format: {}", up_to);
            return false;
        }
    };

    let local_now = current_time.with_timezone(&tz);
    let today = local_now.date_naive();

    // Create datetime objects for start and end times
    let start_dt = today.and_time(start_time).and_local_timezone(tz).unwrap();
    let up_to_dt = today.and_time(up_to_time).and_local_timezone(tz).unwrap();

    // Handle case where time range spans across midnight
    if start_time > up_to_time {
        // If current time is after start time OR before up_to time
        local_now >= start_dt || local_now <= up_to_dt
    } else {
        // Normal case: current time is between start and up_to
        local_now >= start_dt && local_now <= up_to_dt
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        chrono::{TimeZone, Utc},
    };

    #[test]
    fn test_is_now_between() {
        // Test case 1: Current time is between start and up_to
        let timezone = "America/New_York";
        let start = "10:00:00";
        let up_to = "18:00:00";
        let current_time = Utc.with_ymd_and_hms(2024, 3, 15, 14, 0, 0).unwrap(); // 2 PM UTC
        let result = is_now_between(timezone, start, up_to, current_time);
        assert!(
            result,
            "Should be true when current time is between start and up_to"
        );

        // Test case 2: Current time is before start
        let timezone = "America/New_York";
        let start = "20:00:00";
        let up_to = "22:00:00";
        let current_time = Utc.with_ymd_and_hms(2024, 3, 15, 14, 0, 0).unwrap(); // 2 PM UTC
        let result = is_now_between(timezone, start, up_to, current_time);
        assert!(!result, "Should be false when current time is before start");

        // Test case 3: Current time is after up_to
        let timezone = "America/New_York";
        let start = "00:00:00";
        let up_to = "06:00:00";
        let current_time = Utc.with_ymd_and_hms(2024, 3, 15, 14, 0, 0).unwrap(); // 2 PM UTC
        let result = is_now_between(timezone, start, up_to, current_time);
        assert!(!result, "Should be false when current time is after up_to");

        // Test case 4: Invalid timezone
        let timezone = "Invalid/Timezone";
        let start = "10:00:00";
        let up_to = "18:00:00";
        let current_time = Utc.with_ymd_and_hms(2024, 3, 15, 14, 0, 0).unwrap();
        let result = is_now_between(timezone, start, up_to, current_time);
        assert!(!result, "Should return false for invalid timezone");

        // Test case 5: Invalid time format
        let timezone = "America/New_York";
        let start = "25:00:00"; // Invalid hour
        let up_to = "18:00:00";
        let current_time = Utc.with_ymd_and_hms(2024, 3, 15, 14, 0, 0).unwrap();
        let result = is_now_between(timezone, start, up_to, current_time);
        assert!(!result, "Should return false for invalid time format");

        // Test case 6: Time range spanning midnight
        let timezone = "America/New_York";
        let start = "23:00:00";
        let up_to = "01:00:00";
        let current_time = Utc.with_ymd_and_hms(2024, 3, 15, 4, 0, 0).unwrap(); // 4 AM UTC (midnight NY)
        let result = is_now_between(timezone, start, up_to, current_time);
        assert!(
            result,
            "Should be true when current time is within midnight-spanning range"
        );
    }
}
