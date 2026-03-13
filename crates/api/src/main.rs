mod api;
mod config;
mod handlers;
mod repositories;
mod services;
mod state;

use std::sync::Arc;

use anyhow::Result;
use proto::scheduler::scheduler_service_client::SchedulerServiceClient;
use tokio::sync::Mutex;
use tonic::transport::Endpoint;

use config::AppConfig;
use repositories::secrets::new_secrets_repository;
use services::secrets::new_secrets_service;
use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load()?;

    let channel = Endpoint::new(config.scheduler.address())?.connect_lazy();
    let scheduler_client = Arc::new(Mutex::new(SchedulerServiceClient::new(channel)));

    let repository = new_secrets_repository(&config).await?;
    let secrets_service = new_secrets_service(Box::new(repository), scheduler_client);

    let state = Arc::new(AppState {
        config,
        secrets_service: Box::new(secrets_service),
    });

    api::serve(state).await?;

    Ok(())
}
