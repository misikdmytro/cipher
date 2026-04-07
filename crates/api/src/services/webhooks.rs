use chrono::NaiveDateTime;
use proto::notificator::{
    DeleteWebhookRequest, ListWebhooksRequest, RegisterWebhookRequest,
    notificator_service_client::NotificatorServiceClient,
};
use tonic::transport::Channel;
use tracing::error;
use uuid::Uuid;

use crate::services::types::{ServiceError, ServiceResult};

pub struct WebhookInfo {
    pub id: Uuid,
    pub url: String,
    pub created_at: NaiveDateTime,
}

pub struct WebhooksPage {
    pub items: Vec<WebhookInfo>,
    pub total: i64,
}

#[async_trait::async_trait]
pub trait WebhooksService: Send + Sync {
    async fn register_webhook(&self, secret_id: Uuid, url: String) -> ServiceResult<Uuid>;
    async fn list_webhooks(
        &self,
        secret_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ServiceResult<WebhooksPage>;
    async fn delete_webhook(&self, webhook_id: Uuid) -> ServiceResult<()>;
}

struct WebhooksServiceImpl {
    notificator: NotificatorServiceClient<Channel>,
}

pub fn new_webhooks_service(
    notificator: NotificatorServiceClient<Channel>,
) -> impl WebhooksService + 'static {
    WebhooksServiceImpl { notificator }
}

#[async_trait::async_trait]
impl WebhooksService for WebhooksServiceImpl {
    async fn register_webhook(&self, secret_id: Uuid, url: String) -> ServiceResult<Uuid> {
        let result = {
            let mut client = self.notificator.clone();
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

    async fn list_webhooks(
        &self,
        secret_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ServiceResult<WebhooksPage> {
        let result = {
            let mut client = self.notificator.clone();
            client
                .list_webhooks(ListWebhooksRequest {
                    secret_id: secret_id.to_string(),
                    limit,
                    offset,
                })
                .await
        };

        match result {
            Ok(response) => {
                let inner = response.into_inner();
                let mut items = Vec::with_capacity(inner.webhooks.len());
                for entry in inner.webhooks {
                    let id = Uuid::parse_str(&entry.id).map_err(|e| {
                        error!(error = ?e, "Invalid webhook id from notificator");
                        ServiceError::Other("invalid webhook id from notificator".into())
                    })?;
                    let created_at =
                        NaiveDateTime::parse_from_str(&entry.created_at, "%Y-%m-%d %H:%M:%S%.f")
                            .map_err(|e| {
                                error!(error = ?e, "Invalid created_at from notificator");
                                ServiceError::Other("invalid created_at from notificator".into())
                            })?;
                    items.push(WebhookInfo {
                        id,
                        url: entry.url,
                        created_at,
                    });
                }
                Ok(WebhooksPage {
                    items,
                    total: inner.total,
                })
            }
            Err(e) => {
                error!(error = ?e, "Failed to list webhooks via notificator");
                Err(ServiceError::Other("failed to list webhooks".into()))
            }
        }
    }

    async fn delete_webhook(&self, webhook_id: Uuid) -> ServiceResult<()> {
        let result = {
            let mut client = self.notificator.clone();
            client
                .delete_webhook(DeleteWebhookRequest {
                    webhook_id: webhook_id.to_string(),
                })
                .await
        };

        match result {
            Ok(_) => Ok(()),
            Err(e) if e.code() == tonic::Code::NotFound => Err(ServiceError::NotFound),
            Err(e) => {
                error!(error = ?e, "Failed to delete webhook via notificator");
                Err(ServiceError::Other("failed to delete webhook".into()))
            }
        }
    }
}
