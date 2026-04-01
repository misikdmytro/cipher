use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use validator::Validate;

use crate::{
    handlers::common::ErrorResponse,
    handlers::models::webhooks::{RegisterWebhookRequest, RegisterWebhookResponse},
    state::AppState,
};

#[utoipa::path(
    post,
    operation_id = "registerWebhook",
    tag = "webhooks",
    summary = "Register a webhook",
    description = "Registers a webhook URL that will receive POST notifications when rotation events occur for the given secret.",
    path = "/secrets/{secret_id}/webhooks",
    params(
        ("secret_id" = uuid::Uuid, Path, description = "The ID of the secret to register the webhook for")
    ),
    request_body = RegisterWebhookRequest,
    responses(
        (status = 201, description = "Webhook registered successfully", body = RegisterWebhookResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn register_webhook(
    State(state): State<Arc<AppState>>,
    Path(secret_id): Path<uuid::Uuid>,
    Json(body): Json<RegisterWebhookRequest>,
) -> impl IntoResponse {
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let error_response: ErrorResponse = e.into();
            return (StatusCode::BAD_REQUEST, Json(error_response)).into_response();
        }
    }

    match state
        .webhooks_service
        .register_webhook(secret_id, body.url)
        .await
    {
        Ok(id) => (StatusCode::CREATED, Json(RegisterWebhookResponse { id })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in register_webhook");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Internal server error".to_string(),
                }),
            )
                .into_response()
        }
    }
}
