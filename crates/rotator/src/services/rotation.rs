use proto::api::{GetSecretRequest, api_service_client::ApiServiceClient};
use tonic::transport::Channel;
use tracing::{error, info};
use uuid::Uuid;

use crate::helpers::aws::{AwsSecretError, AwsSecretsClient};
use crate::helpers::generator::SecretGenerator;
use crate::services::types::{ServiceError, ServiceResult};

#[async_trait::async_trait]
pub trait RotationService: Send + Sync {
    async fn rotate(&self, secret_id: Uuid) -> ServiceResult<()>;
}

struct RotationServiceImpl {
    api_client: ApiServiceClient<Channel>,
    aws_client: Box<dyn AwsSecretsClient>,
    secret_generator: Box<dyn SecretGenerator>,
}

pub fn new_rotation_service(
    api_client: ApiServiceClient<Channel>,
    aws_client: Box<dyn AwsSecretsClient>,
    secret_generator: Box<dyn SecretGenerator>,
) -> impl RotationService + 'static {
    RotationServiceImpl {
        api_client,
        aws_client,
        secret_generator,
    }
}

#[async_trait::async_trait]
impl RotationService for RotationServiceImpl {
    async fn rotate(&self, secret_id: Uuid) -> ServiceResult<()> {
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

        let path = response.into_inner().path;
        let new_value = self.secret_generator.generate();

        info!(path = %path, "Writing new secret value to AWS Secrets Manager");

        match self.aws_client.put_secret(&path, &new_value).await {
            Ok(()) => Ok(()),
            Err(AwsSecretError::NotFound) => {
                info!(path = %path, "Secret not found in AWS, creating new secret");
                self.aws_client
                    .create_secret(&path, &new_value)
                    .await
                    .map_err(|e| {
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
}
