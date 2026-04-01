use std::sync::Mutex;

use api::services::rotation::{RotationOutcome, RotationTriggerService};
use api::services::types::{ServiceError, ServiceResult};
use proto::notificator::{
    RegisterWebhookRequest, RegisterWebhookResponse, notificator_service_server::NotificatorService,
};
use proto::scheduler::{ScheduleSecretRotationRequest, scheduler_service_server::SchedulerService};
use uuid::Uuid;

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

pub struct MockNotificatorService {
    should_fail: Mutex<bool>,
    received: Mutex<Vec<RegisterWebhookRequest>>,
}

impl MockNotificatorService {
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

    pub fn received_requests(&self) -> Vec<RegisterWebhookRequest> {
        self.received.lock().unwrap().clone()
    }
}

#[tonic::async_trait]
impl NotificatorService for MockNotificatorService {
    async fn register_webhook(
        &self,
        request: tonic::Request<RegisterWebhookRequest>,
    ) -> Result<tonic::Response<RegisterWebhookResponse>, tonic::Status> {
        let req = request.into_inner();
        self.received.lock().unwrap().push(req);

        if *self.should_fail.lock().unwrap() {
            return Err(tonic::Status::internal("mock notificator failure"));
        }

        Ok(tonic::Response::new(RegisterWebhookResponse {
            webhook_id: Uuid::new_v4().to_string(),
        }))
    }
}

pub struct MockRotationTriggerService {
    result: Mutex<Option<ServiceResult<RotationOutcome>>>,
    received: Mutex<Vec<Uuid>>,
}

impl MockRotationTriggerService {
    pub fn success_single(path: &str) -> Self {
        Self {
            result: Mutex::new(Some(Ok(RotationOutcome::Single {
                path: path.to_string(),
            }))),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn success_blue_green(
        active_slot: &str,
        active_path: &str,
        ready_slot: &str,
        ready_path: &str,
    ) -> Self {
        Self {
            result: Mutex::new(Some(Ok(RotationOutcome::BlueGreen {
                active_slot: active_slot.to_string(),
                active_path: active_path.to_string(),
                ready_slot: ready_slot.to_string(),
                ready_path: ready_path.to_string(),
            }))),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn not_found() -> Self {
        Self {
            result: Mutex::new(Some(Err(ServiceError::NotFound))),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn failing() -> Self {
        Self {
            result: Mutex::new(Some(Err(ServiceError::Other(
                "rotation failed".to_string(),
            )))),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn received_requests(&self) -> Vec<Uuid> {
        self.received.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl RotationTriggerService for MockRotationTriggerService {
    async fn rotate_secret(&self, secret_id: Uuid) -> ServiceResult<RotationOutcome> {
        self.received.lock().unwrap().push(secret_id);
        self.result
            .lock()
            .unwrap()
            .take()
            .expect("MockRotationTriggerService: no result configured")
    }
}
