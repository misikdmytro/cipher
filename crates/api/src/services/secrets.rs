use std::sync::Arc;

use interfaces::secrets::Secret;
use proto::scheduler::{
    ScheduleSecretRotationRequest, scheduler_service_client::SchedulerServiceClient,
};
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tracing::error;
use uuid::Uuid;

use crate::{
    repositories::secrets::{
        AddSecretRequest, DeleteSecretRequest, GetSecretError, SecretsRepository,
    },
    services::types::{ServiceError, ServiceResult},
};

#[async_trait::async_trait]
pub trait SecretsService: Send + Sync {
    async fn get_secret_by_id(&self, id: Uuid) -> ServiceResult<Secret>;
    async fn create_secret(&self, path: String, cron_expression: String) -> ServiceResult<Uuid>;
}

struct SecretsServiceImpl {
    repository: Box<dyn SecretsRepository>,
    scheduler: Arc<Mutex<SchedulerServiceClient<Channel>>>,
}

pub fn new_secrets_service(
    repository: Box<dyn SecretsRepository>,
    scheduler: Arc<Mutex<SchedulerServiceClient<Channel>>>,
) -> impl SecretsService + 'static {
    SecretsServiceImpl {
        repository,
        scheduler,
    }
}

#[async_trait::async_trait]
impl SecretsService for SecretsServiceImpl {
    async fn get_secret_by_id(&self, id: Uuid) -> ServiceResult<Secret> {
        self.repository
            .get_secret_by_id(id)
            .await
            .map_err(|e| match e {
                GetSecretError::NotFound => ServiceError::NotFound,
                GetSecretError::Infrastructure(e) => {
                    error!(error = ?e, "Database error while fetching secret");
                    ServiceError::PersistenceError("database error".into())
                }
            })
    }

    async fn create_secret(&self, path: String, cron_expression: String) -> ServiceResult<Uuid> {
        let request = AddSecretRequest {
            path,
            cron_expression: cron_expression.clone(),
        };
        let secret = self.repository.save_secret(request).await.map_err(|e| {
            error!(error = ?e, "Failed to save secret");
            ServiceError::PersistenceError("failed to save secret".into())
        })?;

        let result = {
            let mut scheduler = self.scheduler.lock().await;
            scheduler
                .schedule_secret_rotation(ScheduleSecretRotationRequest {
                    secret_id: secret.id.to_string(),
                    cron_expression,
                })
                .await
        };

        match result {
            Ok(_) => Ok(secret.id),
            Err(e) => {
                error!(error = ?e, "Failed to schedule secret rotation, rolling back");

                let request = DeleteSecretRequest { id: secret.id };
                self.repository
                    .delete_secret(request)
                    .await
                    .inspect_err(|e| {
                        error!(error = ?e, "Failed to roll back secret after scheduling failure");
                    })
                    .ok();

                Err(ServiceError::Other(
                    "failed to schedule secret rotation".into(),
                ))
            }
        }
    }
}
