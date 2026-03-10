use anyhow::Result;
use proto::scheduler::scheduler_service_server::SchedulerServiceServer;
use tonic::transport::Server;

use crate::grpc::scheduler::new_scheduler_service;

mod grpc;

#[tokio::main]
async fn main() -> Result<()> {
    let scheduler_server = new_scheduler_service();
    let addr = "[::1]:50052".parse()?; // TODO: implement config here
    let scheduler = SchedulerServiceServer::new(scheduler_server);
    Server::builder()
        .add_service(scheduler)
        .serve(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start gRPC server: {}", e))?;

    Ok(())
}
