use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use interfaces::secrets::{
    ActiveSlot, AwsProviderConfig, BlueGreenStrategyConfig, ProviderConfig, SingleStrategyConfig,
    StrategyConfig,
};
use std::sync::Arc;
use validator::Validate;

use crate::{
    handlers::common::ErrorResponse,
    handlers::models::pagination::PaginatedResponse,
    handlers::models::pagination::PaginationParams,
    handlers::models::secrets::{
        ActivateBlueGreenResponse, BlueGreenRotationResponse, ProviderRequest,
        RotateSecretResponse, RotationDetailsResponse, RotationStatus, SaveSecretRequest,
        SaveSecretResponse, SecretResponse, SingleRotationResponse, StrategyRequest,
    },
    services::{rotation::RotationOutcome, types::ServiceError},
    state::AppState,
};

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

    let strategy = match body.strategy {
        StrategyRequest::Single(s) => StrategyConfig::Single(SingleStrategyConfig { path: s.path }),
        StrategyRequest::BlueGreen(bg) => StrategyConfig::BlueGreen(BlueGreenStrategyConfig {
            blue_path: bg.blue_path,
            green_path: bg.green_path,
            active_slot: ActiveSlot::Blue,
        }),
    };

    let provider = match body.provider {
        ProviderRequest::Aws(aws) => ProviderConfig::Aws(AwsProviderConfig {
            role_arn: aws.role_arn,
        }),
    };

    match state
        .secrets_service
        .create_secret(body.cron_expression, provider, strategy)
        .await
    {
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

#[utoipa::path(
    get,
    operation_id = "listSecrets",
    tag = "secrets",
    summary = "List secrets",
    description = "Returns a paginated list of registered secrets.",
    path = "/secrets",
    params(
        ("limit" = Option<u64>, Query, description = "Maximum number of results to return (default 20, max 100)"),
        ("offset" = Option<u64>, Query, description = "Number of results to skip (default 0)")
    ),
    responses(
        (status = 200, description = "Secrets retrieved successfully", body = inline(PaginatedResponse<SecretResponse>)),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn list_secrets(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0);

    match state
        .secrets_service
        .list_secrets(limit as i64, offset as i64)
        .await
    {
        Ok(page) => {
            let response = PaginatedResponse {
                items: page.items.into_iter().map(SecretResponse::from).collect(),
                total: page.total as u64,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in list_secrets");
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
    operation_id = "getSecret",
    tag = "secrets",
    summary = "Get a secret by ID",
    description = "Returns the metadata for a single registered secret.",
    path = "/secrets/{id}",
    params(
        ("id" = uuid::Uuid, Path, description = "The ID of the secret")
    ),
    responses(
        (status = 200, description = "Secret retrieved successfully", body = SecretResponse),
        (status = 404, description = "Secret not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn get_secret_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match state.secrets_service.get_secret_by_id(id).await {
        Ok(secret) => (StatusCode::OK, Json(SecretResponse::from(secret))).into_response(),
        Err(ServiceError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: "Secret not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in get_secret_by_id");
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
    post,
    operation_id = "activateBlueGreen",
    tag = "secrets",
    summary = "Activate blue/green slot",
    description = "Flips the active slot for a blue_green strategy secret and publishes rotation.done.",
    path = "/secrets/{id}/activate",
    params(
        ("id" = uuid::Uuid, Path, description = "The ID of the secret")
    ),
    responses(
        (status = 200, description = "Slot activated successfully", body = ActivateBlueGreenResponse),
        (status = 400, description = "Secret does not use blue_green strategy", body = ErrorResponse),
        (status = 404, description = "Secret not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn activate_blue_green(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    match state.secrets_service.activate_blue_green(id).await {
        Ok(new_slot) => {
            let active_slot = match new_slot {
                ActiveSlot::Blue => "blue".to_string(),
                ActiveSlot::Green => "green".to_string(),
            };
            (
                StatusCode::OK,
                Json(ActivateBlueGreenResponse { active_slot }),
            )
                .into_response()
        }
        Err(ServiceError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: "Secret not found".to_string(),
            }),
        )
            .into_response(),
        Err(ServiceError::InvalidOperation(msg)) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { message: msg }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in activate_blue_green");
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
    post,
    operation_id = "rotateSecret",
    tag = "secrets",
    summary = "Trigger manual rotation",
    description = "Triggers an immediate, synchronous rotation of the secret.",
    path = "/secrets/{id}/rotate",
    params(
        ("id" = uuid::Uuid, Path, description = "The ID of the secret to rotate")
    ),
    responses(
        (status = 200, description = "Rotation completed successfully", body = RotateSecretResponse),
        (status = 404, description = "Secret not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn rotate_secret(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    if let Err(ServiceError::NotFound) = state.secrets_service.get_secret_by_id(id).await {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: "Secret not found".to_string(),
            }),
        )
            .into_response();
    }

    match state.rotation_service.rotate_secret(id).await {
        Ok(outcome) => {
            let (status, details) = match outcome {
                RotationOutcome::Single { path } => (
                    RotationStatus::Done,
                    RotationDetailsResponse::Single(SingleRotationResponse { path }),
                ),
                RotationOutcome::BlueGreen {
                    active_slot,
                    active_path,
                    ready_slot,
                    ready_path,
                } => (
                    RotationStatus::Ready,
                    RotationDetailsResponse::BlueGreen(BlueGreenRotationResponse {
                        active_slot,
                        active_path,
                        ready_slot,
                        ready_path,
                    }),
                ),
            };
            (
                StatusCode::OK,
                Json(RotateSecretResponse { status, details }),
            )
                .into_response()
        }
        Err(ServiceError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                message: "Secret not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = ?e, "Unexpected error in rotate_secret");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    message: "Rotation failed".to_string(),
                }),
            )
                .into_response()
        }
    }
}
