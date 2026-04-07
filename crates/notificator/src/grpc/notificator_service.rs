use std::sync::Arc;

use proto::notificator::{
    DeleteWebhookRequest, DeleteWebhookResponse, ListWebhooksRequest, ListWebhooksResponse,
    RegisterWebhookRequest, RegisterWebhookResponse, WebhookEntry,
    notificator_service_server::NotificatorService,
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::repositories::webhooks::{self, AddWebhookRequest};
use crate::state::AppState;

pub(in crate::grpc) struct NotificatorGrpcService {
    state: Arc<AppState>,
}

pub fn new_notificator_grpc_service(state: Arc<AppState>) -> impl NotificatorService + 'static {
    NotificatorGrpcService { state }
}

#[tonic::async_trait]
impl NotificatorService for NotificatorGrpcService {
    async fn register_webhook(
        &self,
        request: Request<RegisterWebhookRequest>,
    ) -> Result<Response<RegisterWebhookResponse>, Status> {
        let inner = request.into_inner();

        let secret_id = Uuid::parse_str(&inner.secret_id)
            .map_err(|_| Status::invalid_argument("invalid secret_id"))?;

        if inner.url.is_empty() {
            return Err(Status::invalid_argument("url cannot be empty"));
        }

        let webhook = self
            .state
            .webhooks_repository
            .save_webhook(AddWebhookRequest {
                secret_id,
                url: inner.url,
            })
            .await
            .map_err(|e| Status::internal(format!("failed to save webhook: {e}")))?;

        Ok(Response::new(RegisterWebhookResponse {
            webhook_id: webhook.id.to_string(),
        }))
    }

    async fn list_webhooks(
        &self,
        request: Request<ListWebhooksRequest>,
    ) -> Result<Response<ListWebhooksResponse>, Status> {
        let inner = request.into_inner();

        let secret_id = Uuid::parse_str(&inner.secret_id)
            .map_err(|_| Status::invalid_argument("invalid secret_id"))?;

        let page = self
            .state
            .webhooks_repository
            .list_webhooks(webhooks::ListWebhooksRequest {
                secret_id,
                limit: inner.limit,
                offset: inner.offset,
            })
            .await
            .map_err(|e| Status::internal(format!("failed to list webhooks: {e}")))?;

        let entries = page
            .items
            .into_iter()
            .map(|w| WebhookEntry {
                id: w.id.to_string(),
                secret_id: w.secret_id.to_string(),
                url: w.url,
                created_at: w.created_at.to_string(),
            })
            .collect();

        Ok(Response::new(ListWebhooksResponse {
            webhooks: entries,
            total: page.total,
        }))
    }

    async fn delete_webhook(
        &self,
        request: Request<DeleteWebhookRequest>,
    ) -> Result<Response<DeleteWebhookResponse>, Status> {
        let inner = request.into_inner();

        let webhook_id = Uuid::parse_str(&inner.webhook_id)
            .map_err(|_| Status::invalid_argument("invalid webhook_id"))?;

        let deleted = self
            .state
            .webhooks_repository
            .delete_webhook(webhook_id)
            .await
            .map_err(|e| Status::internal(format!("failed to delete webhook: {e}")))?;

        if !deleted {
            return Err(Status::not_found("webhook not found"));
        }

        Ok(Response::new(DeleteWebhookResponse {}))
    }
}
