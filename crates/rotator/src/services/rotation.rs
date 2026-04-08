use common::rotation_publisher::RotationEventPublisher;
use interfaces::events::rotation::RotationDoneDetails;
use proto::api::get_secret_response::{Provider, Strategy};
use proto::api::{GetSecretRequest, api_service_client::ApiServiceClient};
use tonic::transport::Channel;
use tracing::{error, info};
use uuid::Uuid;

use crate::helpers::aws::{AwsSecretError, AwsSecretsClientFactory};
use crate::helpers::generator::SecretGenerator;
use crate::services::types::{RotationResult, ServiceError, ServiceResult};

#[async_trait::async_trait]
pub trait RotationService: Send + Sync {
    async fn rotate(&self, secret_id: Uuid) -> ServiceResult<RotationResult>;
}

struct RotationServiceImpl {
    api_client: ApiServiceClient<Channel>,
    aws_factory: Box<dyn AwsSecretsClientFactory>,
    secret_generator: Box<dyn SecretGenerator>,
    publisher: Box<dyn RotationEventPublisher>,
}

pub fn new_rotation_service(
    api_client: ApiServiceClient<Channel>,
    aws_factory: Box<dyn AwsSecretsClientFactory>,
    secret_generator: Box<dyn SecretGenerator>,
    publisher: Box<dyn RotationEventPublisher>,
) -> impl RotationService + 'static {
    RotationServiceImpl {
        api_client,
        aws_factory,
        secret_generator,
        publisher,
    }
}

#[async_trait::async_trait]
impl RotationService for RotationServiceImpl {
    async fn rotate(&self, secret_id: Uuid) -> ServiceResult<RotationResult> {
        let mut api_client = self.api_client.clone();

        let response = api_client
            .get_secret(GetSecretRequest {
                secret_id: secret_id.to_string(),
            })
            .await
            .map_err(|e| {
                if e.code() == tonic::Code::NotFound {
                    return ServiceError::NotFound;
                }
                error!(error = ?e, "Failed to fetch secret from API");
                ServiceError::ApiError("failed to fetch secret".into())
            })?;

        let inner = response.into_inner();

        let role_arn = inner.provider.as_ref().map(|p| match p {
            Provider::Aws(aws) => aws.role_arn.as_str(),
        });

        let aws_client = self.aws_factory.create(role_arn).await.map_err(|e| {
            error!(error = ?e, "Failed to create AWS client");
            ServiceError::AwsError("failed to assume role".into())
        })?;

        let strategy = inner.strategy.ok_or_else(|| {
            error!("Secret has no strategy configured");
            ServiceError::ApiError("missing strategy in secret response".into())
        })?;

        let new_value = self.secret_generator.generate();

        match strategy {
            Strategy::Single(s) => {
                info!(path = %s.path, "Writing new secret value to AWS Secrets Manager (single strategy)");

                put_or_create(&*aws_client, &s.path, &new_value).await?;

                if let Err(e) = self
                    .publisher
                    .publish_done(
                        secret_id,
                        RotationDoneDetails::Single {
                            path: s.path.clone(),
                        },
                    )
                    .await
                {
                    error!(error = ?e, "Failed to publish rotation.done for single strategy");
                }

                Ok(RotationResult::Single {
                    path: s.path.clone(),
                })
            }
            Strategy::BlueGreen(bg) => {
                let (inactive_slot, inactive_path, active_slot, active_path) =
                    if bg.active_slot == "blue" {
                        ("green", bg.green_path.clone(), "blue", bg.blue_path.clone())
                    } else {
                        ("blue", bg.blue_path.clone(), "green", bg.green_path.clone())
                    };

                info!(
                    inactive_path = %inactive_path,
                    "Writing new secret value to AWS Secrets Manager (blue_green strategy)"
                );

                put_or_create(&*aws_client, &inactive_path, &new_value).await?;

                if let Err(e) = self
                    .publisher
                    .publish_ready(
                        secret_id,
                        active_slot.to_string(),
                        active_path.clone(),
                        inactive_slot.to_string(),
                        inactive_path.clone(),
                    )
                    .await
                {
                    error!(error = ?e, "Failed to publish rotation.ready for blue_green strategy");
                }

                Ok(RotationResult::BlueGreen {
                    active_slot: active_slot.to_string(),
                    active_path,
                    ready_slot: inactive_slot.to_string(),
                    ready_path: inactive_path,
                })
            }
        }
    }
}

async fn put_or_create(
    aws_client: &dyn crate::helpers::aws::AwsSecretsClient,
    path: &str,
    value: &str,
) -> ServiceResult<()> {
    match aws_client.put_secret(path, value).await {
        Ok(()) => Ok(()),
        Err(AwsSecretError::NotFound) => {
            info!(path = %path, "Secret not found in AWS, creating new secret");
            aws_client.create_secret(path, value).await.map_err(|e| {
                error!(error = ?e, "Failed to create secret in AWS");
                ServiceError::AwsError("failed to create secret".into())
            })
        }
        Err(e) => {
            error!(error = ?e, "Failed to write secret to AWS");
            Err(ServiceError::AwsError("failed to write secret".into()))
        }
    }
}
