use std::ops::Deref;
use std::time::SystemTime;
use std::{str::FromStr, sync::Arc};

use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use chrono::Utc;
use cron::Schedule as CronSchedule;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::jobs::rotate_secret::RotateSecretJob;

#[derive(Clone)]
pub struct RotateSecretsState {
    pub storage: Arc<Mutex<PostgresStorage<RotateSecretJob>>>,
}

pub async fn handle_rotate_secret(
    job: RotateSecretJob,
    state: Data<RotateSecretsState>,
) -> Result<(), BoxDynError> {
    info!(secret_id = %job.secret_id, "Executing secret rotation");

    // TODO: trigger rotation via rotator gRPC / AMQP

    reschedule(job, state.deref()).await?;

    Ok(())
}

async fn reschedule(job: RotateSecretJob, state: &RotateSecretsState) -> Result<(), BoxDynError> {
    let schedule = CronSchedule::from_str(&job.cron_expression)
        .map_err(|e| format!("invalid cron expression '{}': {e}", job.cron_expression))?;

    let next_run_at = schedule
        .upcoming(Utc)
        .next()
        .ok_or("cron expression has no future occurrences")?;

    let run_at = SystemTime::from(next_run_at);
    let task = Task::builder(job).run_at_time(run_at).build();

    let mut storage = state.storage.lock().await;
    storage.push_task(task).await.map_err(|e| {
        error!(error = ?e, "Failed to reschedule rotation job");
        e
    })?;

    Ok(())
}
