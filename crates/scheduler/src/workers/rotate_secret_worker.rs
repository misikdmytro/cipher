use std::ops::Deref;
use std::time::SystemTime;
use std::{str::FromStr, sync::Arc};

use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use chrono::Utc;
use cron::Schedule as CronSchedule;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::jobs::rotate_secret::RotateSecretJob;
use crate::services::publisher::{PublishError, RotationPublisher};

#[derive(Clone)]
pub struct RotateSecretsState {
    pub storage: Arc<Mutex<PostgresStorage<RotateSecretJob>>>,
    pub publisher: Arc<dyn RotationPublisher>,
}

#[derive(Debug, Error)]
pub enum RotateSecretError {
    #[error("invalid cron expression: {0}")]
    InvalidCronExpression(#[from] cron::error::Error),

    #[error("cron expression does not have any future occurrences")]
    NoFutureOccurrences,

    #[error("failed to store the scheduled job")]
    StorageError,

    #[error("publish error: {0}")]
    PublishError(#[from] PublishError),
}

pub async fn handle_rotate_secret(
    job: RotateSecretJob,
    state: Data<RotateSecretsState>,
) -> Result<(), RotateSecretError> {
    info!(secret_id = %job.secret_id, "Executing secret rotation");

    state.publisher.publish(job.secret_id).await?;

    match reschedule(job, state.deref()).await {
        Ok(_) => Ok(()),
        Err(e) => match e {
            RotateSecretError::NoFutureOccurrences => {
                warn!("Cron expression has no future occurrences, not rescheduling");
                Ok(())
            }
            _ => Err(e),
        },
    }
}

async fn reschedule(
    job: RotateSecretJob,
    state: &RotateSecretsState,
) -> Result<(), RotateSecretError> {
    let schedule = CronSchedule::from_str(&job.cron_expression).map_err(RotateSecretError::from)?;

    let next_run_at = schedule
        .upcoming(Utc)
        .next()
        .ok_or(RotateSecretError::NoFutureOccurrences)?;

    let run_at = SystemTime::from(next_run_at);
    let task = Task::builder(job).run_at_time(run_at).build();

    let mut storage = state.storage.lock().await;
    storage.push_task(task).await.map_err(|e| {
        error!(error = ?e, "Failed to reschedule rotation job");
        RotateSecretError::StorageError
    })?;

    Ok(())
}
