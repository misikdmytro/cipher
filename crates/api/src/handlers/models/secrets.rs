use interfaces::secrets::{ActiveSlot, ProviderConfig, StrategyConfig};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

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

fn validate_blue_green_paths(req: &BlueGreenStrategyRequest) -> Result<(), ValidationError> {
    if req.blue_path == req.green_path {
        return Err(ValidationError::new("identical_paths")
            .with_message("blue_path and green_path must be different".into()));
    }
    Ok(())
}

#[derive(Deserialize, ToSchema, Validate)]
pub struct SingleStrategyRequest {
    #[schema(example = "prod/payments/stripe_api_key")]
    #[validate(length(min = 1, max = 255))]
    pub path: String,
}

#[derive(Deserialize, ToSchema, Validate)]
#[validate(schema(function = "validate_blue_green_paths"))]
pub struct BlueGreenStrategyRequest {
    #[schema(example = "prod/payments/stripe_api_key_blue")]
    #[validate(length(min = 1, max = 255))]
    pub blue_path: String,
    #[schema(example = "prod/payments/stripe_api_key_green")]
    #[validate(length(min = 1, max = 255))]
    pub green_path: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StrategyRequest {
    Single(SingleStrategyRequest),
    BlueGreen(BlueGreenStrategyRequest),
}

impl Validate for StrategyRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            StrategyRequest::Single(s) => s.validate(),
            StrategyRequest::BlueGreen(bg) => bg.validate(),
        }
    }
}

#[derive(Deserialize, ToSchema, Validate)]
pub struct AwsConfigRequest {
    #[schema(example = "arn:aws:iam::123456789012:role/SecretRotator")]
    #[validate(length(min = 20, max = 2048))]
    pub role_arn: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRequest {
    Aws(AwsConfigRequest),
}

impl Validate for ProviderRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            ProviderRequest::Aws(aws) => aws.validate(),
        }
    }
}

/// Request payload for creating a new secret entry.
#[derive(Deserialize, ToSchema, Validate)]
pub struct SaveSecretRequest {
    /// Cron expression defining the rotation schedule (7-field: sec min hour dom month dow year).
    #[schema(example = "0 0 * * * * *")]
    #[validate(length(min = 1, max = 255), custom(function = "validate_cron"))]
    pub cron_expression: String,

    /// Rotation strategy.
    #[validate(nested)]
    pub strategy: StrategyRequest,

    /// Provider configuration.
    #[validate(nested)]
    pub provider: ProviderRequest,
}

/// Response returned after successful secret creation.
#[derive(Serialize, ToSchema)]
pub struct SaveSecretResponse {
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: uuid::Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct SingleStrategyResponse {
    pub path: String,
}

#[derive(Serialize, ToSchema)]
pub struct BlueGreenStrategyResponse {
    pub blue_path: String,
    pub green_path: String,
    pub active_slot: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StrategyResponse {
    Single(SingleStrategyResponse),
    BlueGreen(BlueGreenStrategyResponse),
}

#[derive(Serialize, ToSchema)]
pub struct AwsConfigResponse {
    pub role_arn: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderResponse {
    Aws(AwsConfigResponse),
}

/// A single secret entry.
#[derive(Serialize, ToSchema)]
pub struct SecretResponse {
    #[schema(example = "3fa85f64-5717-4562-b3fc-2c963f66afa6")]
    pub id: uuid::Uuid,
    pub strategy: StrategyResponse,
    pub provider: ProviderResponse,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<interfaces::secrets::Secret> for SecretResponse {
    fn from(s: interfaces::secrets::Secret) -> Self {
        let strategy = match s.strategy {
            StrategyConfig::Single(single) => {
                StrategyResponse::Single(SingleStrategyResponse { path: single.path })
            }
            StrategyConfig::BlueGreen(bg) => {
                StrategyResponse::BlueGreen(BlueGreenStrategyResponse {
                    blue_path: bg.blue_path,
                    green_path: bg.green_path,
                    active_slot: match bg.active_slot {
                        ActiveSlot::Blue => "blue".to_string(),
                        ActiveSlot::Green => "green".to_string(),
                    },
                })
            }
        };

        let provider = match s.provider {
            ProviderConfig::Aws(aws) => ProviderResponse::Aws(AwsConfigResponse {
                role_arn: aws.role_arn,
            }),
        };

        Self {
            id: s.id,
            strategy,
            provider,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

/// Response after activating a blue/green slot.
#[derive(Serialize, ToSchema)]
pub struct ActivateBlueGreenResponse {
    pub active_slot: String,
}

#[derive(Serialize, ToSchema)]
pub struct SingleRotationResponse {
    pub path: String,
}

#[derive(Serialize, ToSchema)]
pub struct BlueGreenRotationResponse {
    pub active_slot: String,
    pub active_path: String,
    pub ready_slot: String,
    pub ready_path: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RotationDetailsResponse {
    Single(SingleRotationResponse),
    BlueGreen(BlueGreenRotationResponse),
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RotationStatus {
    Done,
    Ready,
}

#[derive(Serialize, ToSchema)]
pub struct RotateSecretResponse {
    pub status: RotationStatus,
    pub details: RotationDetailsResponse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    fn single_strategy() -> StrategyRequest {
        StrategyRequest::Single(SingleStrategyRequest {
            path: "prod/payments/key".to_string(),
        })
    }

    fn blue_green_strategy() -> StrategyRequest {
        StrategyRequest::BlueGreen(BlueGreenStrategyRequest {
            blue_path: "prod/key-blue".to_string(),
            green_path: "prod/key-green".to_string(),
        })
    }

    fn aws_provider() -> ProviderRequest {
        ProviderRequest::Aws(AwsConfigRequest {
            role_arn: "arn:aws:iam::123456789012:role/Rotator".to_string(),
        })
    }

    fn valid_base(strategy: StrategyRequest, provider: ProviderRequest) -> SaveSecretRequest {
        SaveSecretRequest {
            cron_expression: "0 0 * * * * *".to_string(),
            strategy,
            provider,
        }
    }

    #[test]
    fn valid_single_with_aws() {
        assert!(
            valid_base(single_strategy(), aws_provider())
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn valid_blue_green_with_aws() {
        assert!(
            valid_base(blue_green_strategy(), aws_provider())
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn identical_blue_green_paths_rejected() {
        let bg = BlueGreenStrategyRequest {
            blue_path: "prod/same".to_string(),
            green_path: "prod/same".to_string(),
        };
        let err = bg.validate().unwrap_err();
        let codes: Vec<_> = err
            .field_errors()
            .get("__all__")
            .unwrap()
            .iter()
            .map(|e| e.code.as_ref())
            .collect();
        assert!(codes.contains(&"identical_paths"), "got: {:?}", codes);
    }

    #[test]
    fn invalid_cron_rejected() {
        let req = SaveSecretRequest {
            cron_expression: "not-a-cron".to_string(),
            strategy: single_strategy(),
            provider: aws_provider(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.field_errors().contains_key("cron_expression"));
    }

    #[test]
    fn short_role_arn_rejected() {
        let aws = AwsConfigRequest {
            role_arn: "short".to_string(),
        };
        let err = aws.validate().unwrap_err();
        assert!(err.field_errors().contains_key("role_arn"));
    }
}
