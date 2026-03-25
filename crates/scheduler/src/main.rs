use std::sync::Arc;

use anyhow::Result;
use apalis::prelude::*;
use apalis_postgres::PostgresStorage;
use proto::scheduler::scheduler_service_server::SchedulerServiceServer;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;
use tonic::transport::Server;

use scheduler::{
    amqp,
    config::AppConfig,
    grpc::scheduler::new_scheduler_service,
    jobs::rotate_secret::RotateSecretJob,
    services::publisher::new_rotation_publisher,
    workers::rotate_secret_worker::{RotateSecretsState, handle_rotate_secret},
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load()?;

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database.connection_string())
        .await?;

    apalis_postgres::PostgresStorage::<(), (), ()>::migrations()
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("failed to run database migrations: {e}"))?;

    let storage = PostgresStorage::<RotateSecretJob>::new(&pool);
    let storage_ref = Arc::new(Mutex::new(storage.clone()));

    let amqp_channel = amqp::connect(&config.rabbitmq).await?;

    let publisher = Arc::new(new_rotation_publisher(amqp_channel));

    let addr = config.grpc.bind_address().parse()?;
    let scheduler = SchedulerServiceServer::new(new_scheduler_service(storage_ref.clone()));

    tokio::select! {
        result = Monitor::new().register(move |_| {
            WorkerBuilder::new("rotate-secrets")
                .backend(storage.clone())
                .concurrency(config.worker.concurrency)
                .data(RotateSecretsState { storage: storage_ref.clone(), publisher: publisher.clone() })
                .build(handle_rotate_secret)
        }).run() => {
            result.map_err(|e| anyhow::anyhow!("apalis monitor failed: {e}"))?;
        }
        result = Server::builder().add_service(scheduler).serve(addr) => {
            result.map_err(|e| anyhow::anyhow!("gRPC server failed: {e}"))?;
        }
    }

    Ok(())
}
