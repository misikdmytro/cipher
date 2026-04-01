use proto::rotator::{
    RotateSecretRequest, rotate_secret_response::Result as ProtoResult,
    rotator_service_client::RotatorServiceClient,
};
use tonic::transport::Channel;
use tracing::error;
use uuid::Uuid;

use crate::services::types::{ServiceError, ServiceResult};

#[derive(Debug, Clone)]
pub enum RotationOutcome {
    Single {
        path: String,
    },
    BlueGreen {
        active_slot: String,
        active_path: String,
        ready_slot: String,
        ready_path: String,
    },
}

#[async_trait::async_trait]
pub trait RotationTriggerService: Send + Sync {
    async fn rotate_secret(&self, secret_id: Uuid) -> ServiceResult<RotationOutcome>;
}

struct RotationTriggerServiceImpl {
    rotator: RotatorServiceClient<Channel>,
}

pub fn new_rotation_trigger_service(
    rotator: RotatorServiceClient<Channel>,
) -> impl RotationTriggerService + 'static {
    RotationTriggerServiceImpl { rotator }
}

#[async_trait::async_trait]
impl RotationTriggerService for RotationTriggerServiceImpl {
    async fn rotate_secret(&self, secret_id: Uuid) -> ServiceResult<RotationOutcome> {
        let result = {
            let mut client = self.rotator.clone();
            client
                .rotate_secret(RotateSecretRequest {
                    secret_id: secret_id.to_string(),
                })
                .await
        };

        match result {
            Ok(response) => match response.into_inner().result {
                Some(ProtoResult::Single(s)) => Ok(RotationOutcome::Single { path: s.path }),
                Some(ProtoResult::BlueGreen(bg)) => Ok(RotationOutcome::BlueGreen {
                    active_slot: bg.active_slot,
                    active_path: bg.active_path,
                    ready_slot: bg.ready_slot,
                    ready_path: bg.ready_path,
                }),
                None => {
                    error!("Rotator returned empty result");
                    Err(ServiceError::Other("rotator returned empty result".into()))
                }
            },
            Err(status) if status.code() == tonic::Code::NotFound => Err(ServiceError::NotFound),
            Err(e) => {
                error!(error = ?e, "Failed to trigger rotation via rotator");
                Err(ServiceError::Other("rotation failed".into()))
            }
        }
    }
}
