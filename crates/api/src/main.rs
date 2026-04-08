pub mod http;
pub mod middleware;

use std::sync::Arc;

use anyhow::Result;
use api::config::AppConfig;
use api::grpc;
use api::grpc::api_service::new_api_grpc_service;
use api::handlers;
use api::repositories::secrets::new_secrets_repository;
use api::services::rotation::new_rotation_trigger_service;
use api::services::secrets::new_secrets_service;
use api::services::webhooks::new_webhooks_service;
use api::state::AppState;
use proto::notificator::notificator_service_client::NotificatorServiceClient;
use proto::rotator::rotator_service_client::RotatorServiceClient;
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

    let rotator_channel = Endpoint::new(config.rotator.address())?.connect_lazy();
    let rotator_client = RotatorServiceClient::new(rotator_channel);

    let amqp = common::amqp::connect(&config.rabbitmq).await?;
    let publish_channel = amqp.create_publish_channel().await?;

    let repository = new_secrets_repository(&config).await?;
    let publisher = Box::new(common::rotation_publisher::new_rotation_event_publisher(
        publish_channel,
    ));
    let secrets_service = new_secrets_service(Box::new(repository), scheduler_client, publisher);
    let webhooks_service = new_webhooks_service(notificator_client);
    let rotation_service = new_rotation_trigger_service(rotator_client);

    let state = Arc::new(AppState {
        config: config.clone(),
        secrets_service: Box::new(secrets_service),
        webhooks_service: Box::new(webhooks_service),
        rotation_service: Box::new(rotation_service),
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
