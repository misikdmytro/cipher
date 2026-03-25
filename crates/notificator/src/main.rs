use std::sync::Arc;

use anyhow::Result;
use notificator::amqp;
use notificator::config::AppConfig;
use notificator::consumer;
use notificator::grpc;
use notificator::grpc::notificator_service::new_notificator_grpc_service;
use notificator::repositories::webhooks::new_webhooks_repository;
use notificator::services::webhook_delivery::new_webhook_delivery_service;
use notificator::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load()?;

    let repository = new_webhooks_repository(&config).await?;
    let amqp_channel = amqp::connect(&config.rabbitmq).await?;
    let delivery_service = new_webhook_delivery_service();

    let state = Arc::new(AppState {
        config: config.clone(),
        webhooks_repository: Box::new(repository),
        delivery_service: Box::new(delivery_service),
        amqp_channel,
    });

    let grpc_address = config.grpc.bind_address();
    let grpc_svc = new_notificator_grpc_service(state.clone());

    tokio::select! {
        result = grpc::serve(grpc_svc, &grpc_address) => result?,
        result = consumer::serve(state) => result?,
    }

    Ok(())
}
