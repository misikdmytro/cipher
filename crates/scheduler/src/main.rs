use std::sync::Arc;

use anyhow::Result;
use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use proto::scheduler::scheduler_service_server::SchedulerServiceServer;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;

use scheduler::{
    config::AppConfig,
    grpc::scheduler::new_scheduler_service,
    jobs::rotate_secret::RotateSecretJob,
    workers::rotate_secret_worker::{RotateSecretsState, handle_rotate_secret},
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    config.log.init_tracing();
    let shutdown = common::shutdown::cancellation_token();

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database.connection_string())
        .await?;

    apalis_postgres::PostgresStorage::<(), (), ()>::migrations()
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("failed to run database migrations: {e}"))?;

    let storage = PostgresStorage::<RotateSecretJob>::new(&pool);

    let amqp = common::amqp::connect(&config.rabbitmq).await?;
    let publish_channel = amqp.create_publish_channel().await?;

    let publisher = Arc::new(common::rotation_publisher::new_rotation_event_publisher(
        publish_channel,
    ));

    let addr = config.grpc.bind_address().parse()?;
    let scheduler = SchedulerServiceServer::new(new_scheduler_service(storage.clone()));

    let worker_storage = storage.clone();
    let health_address = config.health.bind_address();

    tokio::select! {
        result = Monitor::new().register(move |_| {
            WorkerBuilder::new("rotate-secrets")
                .backend(worker_storage.clone())
                .concurrency(config.worker.concurrency)
                .data(RotateSecretsState { storage: storage.clone(), publisher: publisher.clone() })
                .build(handle_rotate_secret)
        }).run() => {
            result.map_err(|e| anyhow::anyhow!("apalis monitor failed: {e}"))?;
        }
        result = Server::builder().add_service(scheduler).serve_with_shutdown(addr, shutdown.clone().cancelled_owned()) => {
            result.map_err(|e| anyhow::anyhow!("gRPC server failed: {e}"))?;
        }
        result = common::health::serve_health(&health_address, shutdown.clone()) => {
            result?;
        }
    }

    Ok(())
}
