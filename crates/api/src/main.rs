mod http;

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
use tokio::sync::Mutex;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load()?;

    let scheduler_channel = Endpoint::new(config.scheduler.address())?.connect_lazy();
    let scheduler_client = Arc::new(Mutex::new(SchedulerServiceClient::new(scheduler_channel)));

    let notificator_channel = Endpoint::new(config.notificator.address())?.connect_lazy();
    let notificator_client = Arc::new(Mutex::new(NotificatorServiceClient::new(
        notificator_channel,
    )));

    let repository = new_secrets_repository(&config).await?;
    let secrets_service = new_secrets_service(Box::new(repository), scheduler_client);
    let webhooks_service = new_webhooks_service(notificator_client);

    let state = Arc::new(AppState {
        config: config.clone(),
        secrets_service: Box::new(secrets_service),
        webhooks_service: Box::new(webhooks_service),
    });

    let http_address = state.config.api.address_without_scheme();
    let grpc_address = config.grpc.address_without_scheme();

    let router = handlers::router(state.clone());
    let grpc_svc = new_api_grpc_service(state);

    tokio::select! {
        result = http::serve(router, &http_address) => result?,
        result = grpc::serve(grpc_svc, &grpc_address) => result?,
    }

    Ok(())
}
