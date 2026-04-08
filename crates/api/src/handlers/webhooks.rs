use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use validator::Validate;

use crate::{
    handlers::common::ErrorResponse,
    handlers::models::pagination::{PaginatedResponse, PaginationParams},
    handlers::models::webhooks::{
        RegisterWebhookRequest, RegisterWebhookResponse, WebhookResponse,
    },
    services::types::ServiceError,
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

#[utoipa::path(
    get,
    operation_id = "listWebhooks",
    tag = "webhooks",
    summary = "List webhooks",
    description = "Returns a paginated list of webhooks registered for the given secret.",
    path = "/secrets/{secret_id}/webhooks",
    params(
        ("secret_id" = uuid::Uuid, Path, description = "The ID of the secret"),
        ("limit" = Option<u64>, Query, description = "Maximum number of results to return (default 20, max 100)"),
        ("offset" = Option<u64>, Query, description = "Number of results to skip (default 0)")
    ),
    responses(
        (status = 200, description = "Webhooks retrieved successfully", body = inline(PaginatedResponse<WebhookResponse>)),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn list_webhooks(
    State(state): State<Arc<AppState>>,
    Path(secret_id): Path<uuid::Uuid>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0);

    match state
        .webhooks_service
        .list_webhooks(secret_id, limit as i64, offset as i64)
        .await
    {
        Ok(page) => {
            let items = page
                .items
                .into_iter()
                .map(|w| WebhookResponse {
                    id: w.id,
                    url: w.url,
                    created_at: w.created_at,
                })
                .collect();
            (
                StatusCode::OK,
                Json(PaginatedResponse {
                    items,
                    total: page.total as u64,
                }),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in list_webhooks");
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

#[utoipa::path(
    delete,
    operation_id = "deleteWebhook",
    tag = "webhooks",
    summary = "Delete a webhook",
    description = "Removes a webhook registration by its ID.",
    path = "/secrets/{secret_id}/webhooks/{webhook_id}",
    params(
        ("secret_id" = uuid::Uuid, Path, description = "The ID of the secret"),
        ("webhook_id" = uuid::Uuid, Path, description = "The ID of the webhook to delete")
    ),
    responses(
        (status = 204, description = "Webhook deleted successfully"),
        (status = 404, description = "Webhook not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path((_secret_id, webhook_id)): Path<(uuid::Uuid, uuid::Uuid)>,
) -> impl IntoResponse {
    match state.webhooks_service.delete_webhook(webhook_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(ServiceError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: "Webhook not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in delete_webhook");
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
