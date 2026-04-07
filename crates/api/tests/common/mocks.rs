use std::sync::Mutex;

use api::services::rotation::{RotationOutcome, RotationTriggerService};
use api::services::types::{ServiceError, ServiceResult};
use proto::notificator::{
    DeleteWebhookRequest, DeleteWebhookResponse, ListWebhooksRequest, ListWebhooksResponse,
    RegisterWebhookRequest, RegisterWebhookResponse, WebhookEntry,
    notificator_service_server::NotificatorService,
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
    webhooks: Mutex<Vec<WebhookEntry>>,
}

impl MockNotificatorService {
    pub fn success() -> Self {
        Self {
            should_fail: Mutex::new(false),
            received: Mutex::new(Vec::new()),
            webhooks: Mutex::new(Vec::new()),
        }
    }

    pub fn failing() -> Self {
        Self {
            should_fail: Mutex::new(true),
            received: Mutex::new(Vec::new()),
            webhooks: Mutex::new(Vec::new()),
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

        if *self.should_fail.lock().unwrap() {
            self.received.lock().unwrap().push(req);
            return Err(tonic::Status::internal("mock notificator failure"));
        }

        let webhook_id = Uuid::new_v4();
        self.webhooks.lock().unwrap().push(WebhookEntry {
            id: webhook_id.to_string(),
            secret_id: req.secret_id.clone(),
            url: req.url.clone(),
            created_at: "2026-01-01 00:00:00".to_string(),
        });
        self.received.lock().unwrap().push(req);

        Ok(tonic::Response::new(RegisterWebhookResponse {
            webhook_id: webhook_id.to_string(),
        }))
    }

    async fn list_webhooks(
        &self,
        request: tonic::Request<ListWebhooksRequest>,
    ) -> Result<tonic::Response<ListWebhooksResponse>, tonic::Status> {
        if *self.should_fail.lock().unwrap() {
            return Err(tonic::Status::internal("mock notificator failure"));
        }

        let inner = request.into_inner();
        let all: Vec<WebhookEntry> = self
            .webhooks
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.secret_id == inner.secret_id)
            .cloned()
            .collect();

        let total = all.len() as i64;
        let offset = inner.offset as usize;
        let limit = inner.limit as usize;
        let webhooks = all.into_iter().skip(offset).take(limit).collect();

        Ok(tonic::Response::new(ListWebhooksResponse {
            webhooks,
            total,
        }))
    }

    async fn delete_webhook(
        &self,
        request: tonic::Request<DeleteWebhookRequest>,
    ) -> Result<tonic::Response<DeleteWebhookResponse>, tonic::Status> {
        if *self.should_fail.lock().unwrap() {
            return Err(tonic::Status::internal("mock notificator failure"));
        }

        let webhook_id = request.into_inner().webhook_id;
        let mut webhooks = self.webhooks.lock().unwrap();
        let len_before = webhooks.len();
        webhooks.retain(|w| w.id != webhook_id);

        if webhooks.len() == len_before {
            return Err(tonic::Status::not_found("webhook not found"));
        }

        Ok(tonic::Response::new(DeleteWebhookResponse {}))
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
