use std::sync::Arc;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use interfaces::secrets::{ActiveSlot, ProviderConfig, StrategyConfig};
use proto::api::{
    AwsConfig, BlueGreenStrategyConfig, GetSecretRequest, GetSecretResponse, SingleStrategyConfig,
    api_service_server::ApiService,
    get_secret_response::{Provider, Strategy},
};

use crate::{services::types::ServiceError, state::AppState};

pub(in crate::grpc) struct ApiGrpcService {
    state: Arc<AppState>,
}

pub fn new_api_grpc_service(state: Arc<AppState>) -> impl ApiService + 'static {
    ApiGrpcService { state }
}

#[tonic::async_trait]
impl ApiService for ApiGrpcService {
    async fn get_secret(
        &self,
        request: Request<GetSecretRequest>,
    ) -> Result<Response<GetSecretResponse>, Status> {
        let secret_id = Uuid::parse_str(&request.into_inner().secret_id)
            .map_err(|_| Status::invalid_argument("invalid secret_id"))?;

        match self.state.secrets_service.get_secret_by_id(secret_id).await {
            Ok(secret) => {
                let provider = Some(match secret.provider {
                    ProviderConfig::Aws(aws) => Provider::Aws(AwsConfig {
                        role_arn: aws.role_arn,
                    }),
                });

                let strategy = Some(match secret.strategy {
                    StrategyConfig::Single(s) => {
                        Strategy::Single(SingleStrategyConfig { path: s.path })
                    }
                    StrategyConfig::BlueGreen(bg) => Strategy::BlueGreen(BlueGreenStrategyConfig {
                        blue_path: bg.blue_path,
                        green_path: bg.green_path,
                        active_slot: match bg.active_slot {
                            ActiveSlot::Blue => "blue".to_string(),
                            ActiveSlot::Green => "green".to_string(),
                        },
                    }),
                });

                Ok(Response::new(GetSecretResponse {
                    secret_id: secret.id.to_string(),
                    provider,
                    strategy,
                }))
            }
            Err(ServiceError::NotFound) => Err(Status::not_found("secret not found")),
            Err(ServiceError::PersistenceError(e)) => {
                Err(Status::internal(format!("database error: {e}")))
            }
            Err(e) => Err(Status::internal(format!("error: {e}"))),
        }
    }
}
