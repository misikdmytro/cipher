use interfaces::events::rotation::RotationDoneDetails;
use interfaces::secrets::{ActiveSlot, ProviderConfig, Secret, StrategyConfig};
use proto::scheduler::{
    ScheduleSecretRotationRequest, scheduler_service_client::SchedulerServiceClient,
};
use tonic::transport::Channel as TonicChannel;
use tracing::error;
use uuid::Uuid;

pub use crate::repositories::secrets::SecretsPage;
pub use common::rotation_publisher::RotationEventPublisher;

use crate::{
    repositories::secrets::{
        AddSecretRequest, DeleteSecretRequest, FlipActiveSlotError, GetSecretError,
        ListSecretsRequest, SecretsRepository,
    },
    services::types::{ServiceError, ServiceResult},
};

#[async_trait::async_trait]
pub trait SecretsService: Send + Sync {
    async fn get_secret_by_id(&self, id: Uuid) -> ServiceResult<Secret>;
    async fn list_secrets(&self, limit: i64, offset: i64) -> ServiceResult<SecretsPage>;
    async fn create_secret(
        &self,
        cron_expression: String,
        provider: ProviderConfig,
        strategy: StrategyConfig,
    ) -> ServiceResult<Uuid>;
    async fn activate_blue_green(&self, id: Uuid) -> ServiceResult<ActiveSlot>;
}

struct SecretsServiceImpl {
    repository: Box<dyn SecretsRepository>,
    scheduler: SchedulerServiceClient<TonicChannel>,
    publisher: Box<dyn RotationEventPublisher>,
}

pub fn new_secrets_service(
    repository: Box<dyn SecretsRepository>,
    scheduler: SchedulerServiceClient<TonicChannel>,
    publisher: Box<dyn RotationEventPublisher>,
) -> impl SecretsService + 'static {
    SecretsServiceImpl {
        repository,
        scheduler,
        publisher,
    }
}

#[async_trait::async_trait]
impl SecretsService for SecretsServiceImpl {
    async fn list_secrets(&self, limit: i64, offset: i64) -> ServiceResult<SecretsPage> {
        self.repository
            .list_secrets(ListSecretsRequest { limit, offset })
            .await
            .map_err(|e| {
                error!(error = ?e, "Database error while listing secrets");
                ServiceError::PersistenceError("database error".into())
            })
    }

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

    async fn create_secret(
        &self,
        cron_expression: String,
        provider: ProviderConfig,
        strategy: StrategyConfig,
    ) -> ServiceResult<Uuid> {
        let request = AddSecretRequest {
            cron_expression: cron_expression.clone(),
            strategy,
            provider,
        };
        let secret = self.repository.save_secret(request).await.map_err(|e| {
            error!(error = ?e, "Failed to save secret");
            ServiceError::PersistenceError("failed to save secret".into())
        })?;

        let result = {
            let mut scheduler = self.scheduler.clone();
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

    async fn activate_blue_green(&self, id: Uuid) -> ServiceResult<ActiveSlot> {
        let secret = self.get_secret_by_id(id).await?;

        let bg = match &secret.strategy {
            StrategyConfig::BlueGreen(bg) => bg.clone(),
            StrategyConfig::Single(_) => {
                return Err(ServiceError::InvalidOperation(
                    "activate is only valid for blue_green strategy".into(),
                ));
            }
        };

        let outdated_path = match &bg.active_slot {
            ActiveSlot::Blue => bg.blue_path.clone(),
            ActiveSlot::Green => bg.green_path.clone(),
        };

        let new_slot = self
            .repository
            .flip_active_slot(id)
            .await
            .map_err(|e| match e {
                FlipActiveSlotError::NotFound => ServiceError::NotFound,
                FlipActiveSlotError::Infrastructure(e) => {
                    error!(error = ?e, "Database error while flipping active slot");
                    ServiceError::PersistenceError("failed to flip active slot".into())
                }
            })?;

        let active_slot_str = match &new_slot {
            ActiveSlot::Blue => "blue".to_string(),
            ActiveSlot::Green => "green".to_string(),
        };

        if let Err(e) = self
            .publisher
            .publish_done(
                id,
                RotationDoneDetails::BlueGreen {
                    active_slot: active_slot_str,
                    outdated_path,
                },
            )
            .await
        {
            error!(error = ?e, secret_id = %id, "Failed to publish rotation.done event after activation");
        }

        Ok(new_slot)
    }
}
