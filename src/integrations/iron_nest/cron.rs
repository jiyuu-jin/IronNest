use {
    crate::{integrations::iron_nest::execute_function, server::actions::get_actions_query},
    core::fmt,
    sqlx::PgPool,
    std::{
        fmt::{Debug, Formatter},
        sync::Arc,
    },
    tokio::sync::RwLock,
    tokio_cron_scheduler::{Job, JobScheduler},
};

#[derive(Clone)]
pub struct CronClient {
    job_scheduler: Arc<RwLock<JobScheduler>>,
}

impl Debug for CronClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CronClient").finish()
    }
}

impl CronClient {
    pub async fn new() -> Self {
        Self {
            job_scheduler: Arc::new(RwLock::new(JobScheduler::new().await.unwrap())),
        }
    }

    pub async fn schedule_tasks(&self, pool: &PgPool) -> Result<(), anyhow::Error> {
        let actions = get_actions_query(pool).await?;

        let mut job_scheduler = self.job_scheduler.write().await;
        job_scheduler.shutdown().await?;

        *job_scheduler = JobScheduler::new().await?;

        for action in actions {
            println!("scheduling action: {}", action.fields.cron);
            job_scheduler
                .add(Job::new_async(
                    action.fields.cron.as_ref(),
                    move |_uuid, mut _l| {
                        let function_name = action.fields.function_name.clone();
                        let function_args = action.fields.function_args.clone();
                        Box::pin(async move {
                            println!("Calling {function_name}({function_args})");
                            execute_function(function_name, function_args).await;
                        })
                    },
                )?)
                .await?;
        }

        job_scheduler.start().await?;
        Ok(())
    }
}
