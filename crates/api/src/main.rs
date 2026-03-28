pub mod http;
pub mod middleware;

use std::sync::Arc;

use anyhow::Result;
use api::config::AppConfig;
use api::grpc;
use api::grpc::api_service::new_api_grpc_service;
use api::handlers;
use api::repositories::secrets::new_secrets_repository;
use api::services::secrets::new_secrets_service;
use api::services::webhooks::new_webhooks_service;
use api::state::AppState;
use proto::notificator::notificator_service_client::NotificatorServiceClient;
use proto::scheduler::scheduler_service_client::SchedulerServiceClient;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    config.log.init_tracing();
    let shutdown = common::shutdown::cancellation_token();

    let scheduler_channel = Endpoint::new(config.scheduler.address())?.connect_lazy();
    let scheduler_client = SchedulerServiceClient::new(scheduler_channel);

    let notificator_channel = Endpoint::new(config.notificator.address())?.connect_lazy();
    let notificator_client = NotificatorServiceClient::new(notificator_channel);

    let repository = new_secrets_repository(&config).await?;
    let secrets_service = new_secrets_service(Box::new(repository), scheduler_client);
    let webhooks_service = new_webhooks_service(notificator_client);

    let state = Arc::new(AppState {
        config: config.clone(),
        secrets_service: Box::new(secrets_service),
        webhooks_service: Box::new(webhooks_service),
    });

    let http_address = state.config.api.bind_address();
    let grpc_address = config.grpc.bind_address();

    let router = handlers::router(state.clone());
    let grpc_svc = new_api_grpc_service(state);

    tokio::select! {
        result = http::serve(router, &http_address, shutdown.clone()) => result?,
        result = grpc::serve(grpc_svc, &grpc_address, shutdown.clone()) => result?,
    }

    Ok(())
}
