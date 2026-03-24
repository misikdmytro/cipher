use std::sync::Mutex;

use proto::scheduler::{ScheduleSecretRotationRequest, scheduler_service_server::SchedulerService};

pub struct MockSchedulerService {
    should_fail: Mutex<bool>,
    received: Mutex<Vec<ScheduleSecretRotationRequest>>,
}

impl MockSchedulerService {
    pub fn success() -> Self {
        Self {
            should_fail: Mutex::new(false),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn failing() -> Self {
        Self {
            should_fail: Mutex::new(true),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn received_requests(&self) -> Vec<ScheduleSecretRotationRequest> {
        self.received.lock().unwrap().clone()
    }
}

#[tonic::async_trait]
impl SchedulerService for MockSchedulerService {
    async fn schedule_secret_rotation(
        &self,
        request: tonic::Request<ScheduleSecretRotationRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let req = request.into_inner();
        self.received.lock().unwrap().push(req);

        if *self.should_fail.lock().unwrap() {
            return Err(tonic::Status::internal("mock scheduler failure"));
        }

        Ok(tonic::Response::new(()))
    }
}
