use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{services::types::ServiceError, state::AppState};

#[derive(Deserialize)]
pub(in crate::handlers) struct SaveSecretRequest {
    pub path: String,
}

#[derive(Serialize)]
pub(in crate::handlers) struct SaveSecretResponse {
    pub id: uuid::Uuid,
}

pub(in crate::handlers) async fn save_secret(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveSecretRequest>,
) -> impl IntoResponse {
    match state.secrets_service.create_secret(body.path).await {
        Ok(id) => (StatusCode::CREATED, Json(SaveSecretResponse { id })).into_response(),
        Err(ServiceError::ValidationError(msg)) => {
            (StatusCode::BAD_REQUEST, msg).into_response()
        }
        Err(ServiceError::NotFound(msg)) => (StatusCode::NOT_FOUND, msg).into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in save_secret");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}