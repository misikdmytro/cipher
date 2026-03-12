use std::sync::Arc;

use proto::scheduler::{
    ScheduleSecretRotationRequest, scheduler_service_client::SchedulerServiceClient,
};
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tracing::error;
use uuid::Uuid;

use crate::{
    repositories::secrets::{AddSecretRequest, SecretsRepository},
    services::types::{ServiceError, ServiceResult},
};

#[async_trait::async_trait]
pub trait SecretsService {
    async fn create_secret(&self, path: String) -> ServiceResult<Uuid>;
}

struct SecretsServiceImpl {
    repository: Box<dyn SecretsRepository>,
    scheduler: Arc<Mutex<SchedulerServiceClient<Channel>>>,
}

pub fn new_secrets_service(
    repository: Box<dyn SecretsRepository>,
    scheduler: Arc<Mutex<SchedulerServiceClient<Channel>>>,
) -> impl SecretsService + Send + Sync + 'static {
    SecretsServiceImpl {
        repository,
        scheduler,
    }
}

#[async_trait::async_trait]
impl SecretsService for SecretsServiceImpl {
    async fn create_secret(&self, path: String) -> ServiceResult<Uuid> {
        let request = AddSecretRequest { path };
        let secret = self.repository.save_secret(request).await.map_err(|e| {
            error!(error = ?e, "Failed to save secret");
            ServiceError::PersistenceError("failed to save secret".to_string())
        })?;

        let mut scheduler = self.scheduler.lock().await;
        scheduler
            .schedule_secret_rotation(ScheduleSecretRotationRequest {
                secret_id: secret.id.to_string(),
                cron_expression: "0/5 * * * * * *".to_string(), // TODO: make this configurable
            })
            .await
            .map_err(|e| {
                error!(status = ?e, "Failed to schedule secret rotation");
                ServiceError::Other(anyhow::anyhow!("failed to schedule secret rotation"))
            })?;

        Ok(secret.id)
    }
}
