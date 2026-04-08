use std::str::FromStr;
use std::time::SystemTime;

use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use chrono::Utc;
use cron::Schedule as CronSchedule;
use proto::scheduler::{ScheduleSecretRotationRequest, scheduler_service_server::SchedulerService};
use tonic::{Request, Response, Status};
use tracing::error;
use uuid::Uuid;

use crate::jobs::rotate_secret::RotateSecretJob;

pub struct SchedulerServer {
    storage: PostgresStorage<RotateSecretJob>,
}

pub fn new_scheduler_service(
    storage: PostgresStorage<RotateSecretJob>,
) -> impl SchedulerService + 'static {
    SchedulerServer { storage }
}

#[tonic::async_trait]
impl SchedulerService for SchedulerServer {
    async fn schedule_secret_rotation(
        &self,
        request: Request<ScheduleSecretRotationRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let secret_id = Uuid::parse_str(&req.secret_id)
            .map_err(|_| Status::invalid_argument("invalid secret_id: must be a valid UUID"))?;

        let schedule = CronSchedule::from_str(&req.cron_expression).map_err(|_| {
            Status::invalid_argument(format!("invalid cron expression: {}", req.cron_expression))
        })?;

        let next_run_at = schedule
            .upcoming(Utc)
            .next()
            .ok_or_else(|| Status::invalid_argument("cron expression has no future occurrences"))?;

        let job = RotateSecretJob {
            secret_id,
            cron_expression: req.cron_expression,
        };

        let task = Task::builder(job)
            .run_at_time(SystemTime::from(next_run_at))
            .build();

        self.storage.clone().push_task(task).await.map_err(|e| {
            error!(error = ?e, "Failed to persist rotation schedule");
            Status::internal("internal error")
        })?;

        Ok(Response::new(()))
    }
}
