use proto::scheduler::{ScheduleSecretRotationRequest, scheduler_service_server::SchedulerService};
use tonic::{Request, Response, Status};

struct SchedulerServer;

pub fn new_scheduler_service() -> impl SchedulerService + 'static {
    SchedulerServer
}

#[async_trait::async_trait]
impl SchedulerService for SchedulerServer {
    async fn schedule_secret_rotation(
        &self,
        request: Request<ScheduleSecretRotationRequest>,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }
}
