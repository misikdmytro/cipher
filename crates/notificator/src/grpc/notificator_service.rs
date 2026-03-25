use std::sync::Arc;

use proto::notificator::{
    RegisterWebhookRequest, RegisterWebhookResponse, notificator_service_server::NotificatorService,
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::repositories::webhooks::AddWebhookRequest;
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
}
