use std::sync::Mutex;

use notificator::repositories::models::webhooks::Webhook;
use notificator::repositories::webhooks::{
    AddWebhookError, AddWebhookRequest, GetWebhooksError, WebhooksRepository,
};
use notificator::services::types::WebhookPayload;
use notificator::services::webhook_delivery::DeliveryError;
use notificator::services::webhook_delivery::WebhookDeliveryService;
use uuid::Uuid;

pub struct MockWebhooksRepository {
    webhooks: Mutex<Vec<Webhook>>,
}

impl MockWebhooksRepository {
    pub fn with_webhooks(webhooks: Vec<Webhook>) -> Self {
        Self {
            webhooks: Mutex::new(webhooks),
        }
    }

    pub fn empty() -> Self {
        Self {
            webhooks: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl WebhooksRepository for MockWebhooksRepository {
    async fn save_webhook(&self, request: AddWebhookRequest) -> Result<Webhook, AddWebhookError> {
        let webhook = Webhook {
            id: Uuid::new_v4(),
            secret_id: request.secret_id,
            url: request.url,
            created_at: chrono::Utc::now().naive_utc(),
        };
        self.webhooks.lock().unwrap().push(webhook.clone());
        Ok(webhook)
    }

    async fn get_webhooks_by_secret_id(
        &self,
        secret_id: Uuid,
    ) -> Result<Vec<Webhook>, GetWebhooksError> {
        let webhooks = self
            .webhooks
            .lock()
            .unwrap()
            .iter()
            .filter(|w| w.secret_id == secret_id)
            .cloned()
            .collect();
        Ok(webhooks)
    }
}

pub struct MockWebhookDeliveryService {
    deliver_calls: Mutex<Vec<(String, WebhookPayload)>>,
    should_fail: bool,
}

impl MockWebhookDeliveryService {
    pub fn success() -> Self {
        Self {
            deliver_calls: Mutex::new(Vec::new()),
            should_fail: false,
        }
    }

    pub fn failing() -> Self {
        Self {
            deliver_calls: Mutex::new(Vec::new()),
            should_fail: true,
        }
    }

    pub fn deliver_calls(&self) -> Vec<(String, WebhookPayload)> {
        self.deliver_calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl WebhookDeliveryService for MockWebhookDeliveryService {
    async fn deliver(&self, url: &str, payload: &WebhookPayload) -> Result<(), DeliveryError> {
        self.deliver_calls
            .lock()
            .unwrap()
            .push((url.to_string(), payload.clone()));

        if self.should_fail {
            return Err(DeliveryError::StatusError(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }

        Ok(())
    }
}
