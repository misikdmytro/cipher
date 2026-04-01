use std::sync::Arc;

use proto::rotator::{
    BlueGreenRotationResult, RotateSecretRequest, RotateSecretResponse, SingleRotationResult,
    rotate_secret_response::Result as ProtoResult, rotator_service_server::RotatorService,
};
use tonic::{Request, Response, Status};
use tracing::error;
use uuid::Uuid;

use crate::services::types::{RotationResult, ServiceError};
use crate::state::AppState;

pub(in crate::grpc) struct RotatorGrpcService {
    state: Arc<AppState>,
}

pub fn new_rotator_grpc_service(state: Arc<AppState>) -> impl RotatorService + 'static {
    RotatorGrpcService { state }
}

#[tonic::async_trait]
impl RotatorService for RotatorGrpcService {
    async fn rotate_secret(
        &self,
        request: Request<RotateSecretRequest>,
    ) -> Result<Response<RotateSecretResponse>, Status> {
        let secret_id = Uuid::parse_str(&request.into_inner().secret_id)
            .map_err(|_| Status::invalid_argument("invalid secret_id"))?;

        if let Err(e) = self.state.publisher.publish_started(secret_id).await {
            error!(error = ?e, "Failed to publish rotation.started");
            return Err(Status::internal("failed to publish rotation.started"));
        }

        match self.state.rotation_service.rotate(secret_id).await {
            Ok(result) => {
                let proto_result = match result {
                    RotationResult::Single { path } => {
                        ProtoResult::Single(SingleRotationResult { path })
                    }
                    RotationResult::BlueGreen {
                        active_slot,
                        active_path,
                        ready_slot,
                        ready_path,
                    } => ProtoResult::BlueGreen(BlueGreenRotationResult {
                        active_slot,
                        active_path,
                        ready_slot,
                        ready_path,
                    }),
                };
                Ok(Response::new(RotateSecretResponse {
                    result: Some(proto_result),
                }))
            }
            Err(ServiceError::NotFound) => {
                let _ = self
                    .state
                    .publisher
                    .publish_failed(secret_id, "secret not found".to_string())
                    .await;
                Err(Status::not_found("secret not found"))
            }
            Err(e) => {
                let _ = self
                    .state
                    .publisher
                    .publish_failed(secret_id, e.to_string())
                    .await;
                error!(error = ?e, "Rotation failed");
                Err(Status::internal(format!("rotation failed: {e}")))
            }
        }
    }
}
