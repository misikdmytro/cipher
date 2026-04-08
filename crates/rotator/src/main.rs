use std::sync::Arc;

use anyhow::Result;
use aws_config::BehaviorVersion;
use common::rotation_publisher::new_rotation_event_publisher;
use proto::api::api_service_client::ApiServiceClient;
use rotator::config::AppConfig;
use rotator::consumer;
use rotator::grpc;
use rotator::grpc::rotator_service::new_rotator_grpc_service;
use rotator::helpers::aws::DefaultAwsSecretsClientFactory;
use rotator::helpers::generator::RandomPasswordGenerator;
use rotator::services::rotation::new_rotation_service;
use rotator::state::AppState;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    config.log.init_tracing();
    let shutdown = common::shutdown::cancellation_token();

    let channel = Endpoint::new(config.api.address())?.connect_lazy();
    let api_client = ApiServiceClient::new(channel);

    let aws_cfg = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let aws_factory = Box::new(DefaultAwsSecretsClientFactory::new(aws_cfg));

    let secret_generator = Box::new(RandomPasswordGenerator::new(48));

    let amqp = common::amqp::connect(&config.rabbitmq).await?;
    let publish_channel = amqp.create_publish_channel().await?;
    let consume_channel = amqp.create_consume_channel().await?;

    let publisher = Arc::new(new_rotation_event_publisher(publish_channel.clone()));
    let rotation_service = new_rotation_service(
        api_client,
        aws_factory,
        secret_generator,
        Box::new(new_rotation_event_publisher(publish_channel)),
    );

    let state = Arc::new(AppState {
        rotation_service: Box::new(rotation_service),
        publisher,
        amqp_channel: consume_channel,
    });

    let grpc_address = config.grpc.bind_address();
    let health_address = config.health.bind_address();

    let grpc_svc = new_rotator_grpc_service(state.clone());

    tokio::select! {
        result = consumer::serve(state, shutdown.clone()) => result?,
        result = grpc::serve(grpc_svc, &grpc_address, shutdown.clone()) => result?,
        result = common::health::serve_health(&health_address, shutdown.clone()) => result?,
    }

    Ok(())
}
