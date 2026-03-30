use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

use crate::{handlers::common::ErrorResponse, services::types::ServiceError, state::AppState};

fn validate_cron(expr: &str) -> Result<(), ValidationError> {
    expr.parse::<cron::Schedule>().map(|_| ()).map_err(|_| {
        ValidationError::new("invalid_cron_expression").with_message(
            format!(
                "Invalid cron expression: '{}'. Allowed format: sec min hour dom month dow year",
                expr
            )
            .into(),
        )
    })
}

/// AWS-specific configuration for cross-account role assumption.
#[derive(Deserialize, ToSchema, Validate)]
pub(in crate::handlers) struct AwsConfigRequest {
    /// The ARN of the IAM role to assume before rotating the secret.
    #[schema(example = "arn:aws:iam::123456789012:role/SecretRotator")]
    #[validate(length(min = 20, max = 2048))]
    pub role_arn: String,
}

fn validate_exactly_one_provider(req: &SaveSecretRequest) -> Result<(), ValidationError> {
    let count = [req.aws.is_some()].iter().filter(|&&x| x).count();
    match count {
        1 => Ok(()),
        0 => Err(ValidationError::new("missing_provider")
            .with_message("Exactly one provider must be specified (e.g. \"aws\")".into())),
        _ => Err(ValidationError::new("multiple_providers").with_message(
            "Exactly one provider must be specified, but multiple were given".into(),
        )),
    }
}

/// Request payload for creating a new secret entry.
#[derive(Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validate_exactly_one_provider"))]
pub(in crate::handlers) struct SaveSecretRequest {
    /// Logical path used by the downstream scheduler/rotator services.
    #[schema(example = "prod/payments/stripe_api_key", min_length = 1)]
    #[validate(length(min = 1, max = 255))]
    pub path: String,

    /// Cron expression defining the rotation schedule (7-field: sec min hour dom month dow year).
    #[schema(example = "0 0 * * * * *")]
    #[validate(length(min = 1, max = 255), custom(function = "validate_cron"))]
    pub cron_expression: String,

    /// Optional AWS-specific configuration for cross-account role assumption.
    #[validate(nested)]
    pub aws: Option<AwsConfigRequest>,
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

    match state
        .secrets_service
        .create_secret(
            body.path,
            body.cron_expression,
            body.aws.map(|a| a.role_arn),
        )
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

/// AWS configuration returned in secret responses.
#[derive(Serialize, ToSchema)]
pub(in crate::handlers) struct AwsConfigResponse {
    #[schema(example = "arn:aws:iam::123456789012:role/SecretRotator")]
    pub role_arn: String,
}

/// A single secret entry.
#[derive(Serialize, ToSchema)]
pub(in crate::handlers) struct SecretResponse {
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: uuid::Uuid,
    #[schema(example = "prod/payments/stripe_api_key")]
    pub path: String,
    pub aws: Option<AwsConfigResponse>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<interfaces::secrets::Secret> for SecretResponse {
    fn from(s: interfaces::secrets::Secret) -> Self {
        let aws = s.provider.map(|p| match p {
            interfaces::secrets::ProviderConfig::Aws(a) => AwsConfigResponse {
                role_arn: a.role_arn,
            },
        });

        Self {
            id: s.id,
            path: s.path,
            aws,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

/// Paginated list of secrets.
#[derive(Serialize, ToSchema)]
pub(in crate::handlers) struct ListSecretsResponse {
    pub items: Vec<SecretResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Deserialize)]
pub(in crate::handlers) struct ListSecretsParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    operation_id = "listSecrets",
    tag = "secrets",
    summary = "List secrets",
    description = "Returns a paginated list of registered secrets.",
    path = "/secrets",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of results to return (default 20, max 100)"),
        ("offset" = Option<i64>, Query, description = "Number of results to skip (default 0)")
    ),
    responses(
        (status = 200, description = "Secrets retrieved successfully", body = ListSecretsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub(in crate::handlers) async fn list_secrets(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListSecretsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    match state.secrets_service.list_secrets(limit, offset).await {
        Ok(page) => {
            let response = ListSecretsResponse {
                items: page.items.into_iter().map(SecretResponse::from).collect(),
                total: page.total,
                limit,
                offset,
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

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    fn valid_base(aws: Option<AwsConfigRequest>) -> SaveSecretRequest {
        SaveSecretRequest {
            path: "prod/payments/key".to_string(),
            cron_expression: "0 0 * * * * *".to_string(),
            aws,
        }
    }

    fn aws() -> AwsConfigRequest {
        AwsConfigRequest {
            role_arn: "arn:aws:iam::123456789012:role/Rotator".to_string(),
        }
    }

    #[test]
    fn valid_with_aws() {
        assert!(valid_base(Some(aws())).validate().is_ok());
    }

    #[test]
    fn missing_provider_rejected() {
        let err = valid_base(None).validate().unwrap_err();
        let codes: Vec<_> = err
            .field_errors()
            .get("__all__")
            .unwrap()
            .iter()
            .map(|e| e.code.as_ref())
            .collect();
        assert!(codes.contains(&"missing_provider"), "got: {:?}", codes);
    }

    #[test]
    fn empty_path_rejected() {
        let req = SaveSecretRequest {
            path: "".to_string(),
            cron_expression: "0 0 * * * * *".to_string(),
            aws: Some(aws()),
        };
        let err = req.validate().unwrap_err();
        assert!(err.field_errors().contains_key("path"));
    }

    #[test]
    fn invalid_cron_rejected() {
        let req = SaveSecretRequest {
            path: "prod/key".to_string(),
            cron_expression: "not-a-cron".to_string(),
            aws: Some(aws()),
        };
        let err = req.validate().unwrap_err();
        assert!(err.field_errors().contains_key("cron_expression"));
    }

    #[test]
    fn short_role_arn_rejected() {
        let req = valid_base(Some(AwsConfigRequest {
            role_arn: "short".to_string(),
        }));
        let err = req.validate().unwrap_err();
        let aws_errors = err.errors().get("aws").expect("expected aws errors");
        match aws_errors {
            validator::ValidationErrorsKind::Struct(inner) => {
                assert!(inner.field_errors().contains_key("role_arn"))
            }
            other => panic!("expected Struct errors, got: {:?}", other),
        }
    }
}
