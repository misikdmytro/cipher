use aws_sdk_secretsmanager::{Client as SecretsManagerClient, error::SdkError};

#[derive(Debug, thiserror::Error)]
pub enum AwsSecretError {
    #[error("resource not found")]
    NotFound,

    #[error("{0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait AwsSecretsClient: Send + Sync {
    async fn put_secret(&self, path: &str, value: &str) -> Result<(), AwsSecretError>;
    async fn create_secret(&self, path: &str, value: &str) -> Result<(), AwsSecretError>;
}

pub struct AwsSecretsClientImpl {
    client: SecretsManagerClient,
}

pub fn new_aws_secrets_client(client: SecretsManagerClient) -> impl AwsSecretsClient + 'static {
    AwsSecretsClientImpl { client }
}

#[async_trait::async_trait]
impl AwsSecretsClient for AwsSecretsClientImpl {
    async fn put_secret(&self, path: &str, value: &str) -> Result<(), AwsSecretError> {
        self.client
            .put_secret_value()
            .secret_id(path)
            .secret_string(value)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| match e {
                SdkError::ServiceError(ref se) if se.err().is_resource_not_found_exception() => {
                    AwsSecretError::NotFound
                }
                other => AwsSecretError::Other(other.to_string()),
            })
    }

    async fn create_secret(&self, path: &str, value: &str) -> Result<(), AwsSecretError> {
        self.client
            .create_secret()
            .name(path)
            .secret_string(value)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| AwsSecretError::Other(e.to_string()))
    }
}
