pub mod rotator_service;

use anyhow::Result;
use proto::rotator::rotator_service_server::{RotatorService, RotatorServiceServer};
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;

pub async fn serve(
    svc: impl RotatorService + 'static,
    address: &str,
    shutdown: CancellationToken,
) -> Result<()> {
    let addr = address.parse()?;
    Server::builder()
        .add_service(RotatorServiceServer::new(svc))
        .serve_with_shutdown(addr, shutdown.cancelled_owned())
        .await
        .map_err(|e| anyhow::anyhow!("gRPC server failed: {e}"))
}
