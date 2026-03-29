use std::sync::Arc;

use tonic::{Request, Response, Status};
use uuid::Uuid;

use interfaces::secrets::ProviderConfig;
use proto::api::{
    AwsConfig, GetSecretRequest, GetSecretResponse, api_service_server::ApiService,
    get_secret_response::Provider,
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
                let provider = secret.provider.map(|p| match p {
                    ProviderConfig::Aws(aws) => Provider::Aws(AwsConfig {
                        role_arn: aws.role_arn,
                    }),
                });

                Ok(Response::new(GetSecretResponse {
                    secret_id: secret.id.to_string(),
                    path: secret.path,
                    provider,
                }))
            }
            Err(ServiceError::NotFound) => Err(Status::not_found("secret not found")),
            Err(ServiceError::PersistenceError(e)) => {
                Err(Status::internal(format!("database error: {e}")))
            }
            Err(ServiceError::Other(e)) => Err(Status::internal(format!("other error: {e}"))),
        }
    }
}
