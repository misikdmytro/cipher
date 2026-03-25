use std::sync::Arc;

use proto::notificator::{
    RegisterWebhookRequest, notificator_service_client::NotificatorServiceClient,
};
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tracing::error;
use uuid::Uuid;

use crate::services::types::{ServiceError, ServiceResult};

#[async_trait::async_trait]
pub trait WebhooksService: Send + Sync {
    async fn register_webhook(&self, secret_id: Uuid, url: String) -> ServiceResult<Uuid>;
}

struct WebhooksServiceImpl {
    notificator: Arc<Mutex<NotificatorServiceClient<Channel>>>,
}

pub fn new_webhooks_service(
    notificator: Arc<Mutex<NotificatorServiceClient<Channel>>>,
) -> impl WebhooksService + 'static {
    WebhooksServiceImpl { notificator }
}

#[async_trait::async_trait]
impl WebhooksService for WebhooksServiceImpl {
    async fn register_webhook(&self, secret_id: Uuid, url: String) -> ServiceResult<Uuid> {
        let result = {
            let mut client = self.notificator.lock().await;
            client
                .register_webhook(RegisterWebhookRequest {
                    secret_id: secret_id.to_string(),
                    url,
                })
                .await
        };

        match result {
            Ok(response) => Uuid::parse_str(&response.into_inner().webhook_id).map_err(|e| {
                error!(error = ?e, "Invalid webhook_id from notificator");
                ServiceError::Other("invalid webhook_id from notificator".into())
            }),
            Err(e) => {
                error!(error = ?e, "Failed to register webhook via notificator");
                Err(ServiceError::Other("failed to register webhook".into()))
            }
        }
    }
}
