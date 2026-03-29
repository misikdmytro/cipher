use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::{Client as SecretsManagerClient, error::SdkError};
use aws_sdk_sts::Client as StsClient;
use tracing::info;

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
pub trait AwsSecretsClientFactory: Send + Sync {
    async fn create(
        &self,
        role_arn: Option<&str>,
    ) -> Result<Box<dyn AwsSecretsClient>, AwsSecretError>;
}

pub struct DefaultAwsSecretsClientFactory {
    base_config: aws_config::SdkConfig,
}

impl DefaultAwsSecretsClientFactory {
    pub fn new(base_config: aws_config::SdkConfig) -> Self {
        Self { base_config }
    }
}

#[async_trait::async_trait]
impl AwsSecretsClientFactory for DefaultAwsSecretsClientFactory {
    async fn create(
        &self,
        role_arn: Option<&str>,
    ) -> Result<Box<dyn AwsSecretsClient>, AwsSecretError> {
        match role_arn {
            None => {
                let client = SecretsManagerClient::new(&self.base_config);
                Ok(Box::new(AwsSecretsClientImpl { client }))
            }
            Some(arn) => {
                info!(role_arn = %arn, "Assuming IAM role for secret rotation");

                let sts_client = StsClient::new(&self.base_config);
                let assume_output = sts_client
                    .assume_role()
                    .role_arn(arn)
                    .role_session_name("cipher-rotation")
                    .send()
                    .await
                    .map_err(|e| {
                        AwsSecretError::Other(format!("STS AssumeRole failed for {arn}: {e}"))
                    })?;

                let creds = assume_output.credentials().ok_or_else(|| {
                    AwsSecretError::Other("STS AssumeRole returned no credentials".into())
                })?;

                let provider = aws_sdk_sts::config::Credentials::new(
                    creds.access_key_id(),
                    creds.secret_access_key(),
                    Some(creds.session_token().to_string()),
                    None,
                    "cipher-sts-assumed",
                );

                let assumed_config = aws_config::defaults(BehaviorVersion::latest())
                    .credentials_provider(provider)
                    .region(self.base_config.region().cloned())
                    .load()
                    .await;

                let client = SecretsManagerClient::new(&assumed_config);
                Ok(Box::new(AwsSecretsClientImpl { client }))
            }
        }
    }
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
