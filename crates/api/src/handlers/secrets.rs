use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use validator::Validate;

use crate::{handlers::common::ErrorResponse, state::AppState};

/// Request payload for creating a new secret entry.
#[derive(Deserialize, ToSchema, Validate)]
pub(in crate::handlers) struct SaveSecretRequest {
    /// Logical path used by the downstream scheduler/rotator services.
    #[schema(example = "prod/payments/stripe_api_key", min_length = 1)]
    #[validate(length(min = 1, max = 255))]
    pub path: String,
}

/// Response returned after successful secret creation.
#[derive(Serialize, ToSchema)]
pub(in crate::handlers) struct SaveSecretResponse {
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: uuid::Uuid,
}

#[utoipa::path(
    post,
    operation_id = "createSecret",
    tag = "secrets",
    summary = "Create a secret",
    description = "Creates a secret record and triggers scheduling logic for rotation.",
    path = "/secrets",
    request_body = SaveSecretRequest,
    responses(
        (status = 201, description = "Secret created successfully", body = SaveSecretResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Resource not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn save_secret(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveSecretRequest>,
) -> impl IntoResponse {
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let error_response: ErrorResponse = e.into();
            return (StatusCode::BAD_REQUEST, Json(error_response)).into_response();
        }
    }

    match state.secrets_service.create_secret(body.path).await {
        Ok(id) => (StatusCode::CREATED, Json(SaveSecretResponse { id })).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in save_secret");
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
