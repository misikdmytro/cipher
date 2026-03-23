mod api;

use std::sync::Arc;

use anyhow::Result;
use proto::scheduler::scheduler_service_client::SchedulerServiceClient;
use tokio::sync::Mutex;
use tonic::transport::Endpoint;

use ::api::config::AppConfig;
use ::api::handlers;
use ::api::repositories::secrets::new_secrets_repository;
use ::api::services::secrets::new_secrets_service;
use ::api::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load()?;

    let channel = Endpoint::new(config.scheduler.address())?.connect_lazy();
    let scheduler_client = Arc::new(Mutex::new(SchedulerServiceClient::new(channel)));

    let repository = new_secrets_repository(&config).await?;
    let secrets_service = new_secrets_service(Box::new(repository), scheduler_client);

    let state = Arc::new(AppState {
        config: config.clone(),
        secrets_service: Box::new(secrets_service),
    });

    let address = state.config.api.address_without_scheme();
    let router = handlers::router(state);
    api::serve(router, &address).await?;

    Ok(())
}
