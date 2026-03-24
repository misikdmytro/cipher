use std::sync::Arc;

use anyhow::Result;
use aws_config::BehaviorVersion;
use proto::api::api_service_client::ApiServiceClient;
use rotator::config::AppConfig;
use rotator::consumer;
use rotator::helpers::aws::new_aws_secrets_client;
use rotator::helpers::generator::RandomHexGenerator;
use rotator::services::rotation::new_rotation_service;
use rotator::state::AppState;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load()?;

    let channel = Endpoint::new(config.api.address())?.connect_lazy();
    let api_client = ApiServiceClient::new(channel);

    let aws_cfg = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let aws_client = Box::new(new_aws_secrets_client(aws_sdk_secretsmanager::Client::new(
        &aws_cfg,
    )));

    let secret_generator = Box::new(RandomHexGenerator::new(32));
    let rotation_service = new_rotation_service(api_client, aws_client, secret_generator);

    let state = Arc::new(AppState {
        config,
        rotation_service: Box::new(rotation_service),
    });

    consumer::serve(state).await?;

    Ok(())
}
