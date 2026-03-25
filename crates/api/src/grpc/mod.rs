pub mod api_service;

use anyhow::Result;
use proto::api::api_service_server::{ApiService, ApiServiceServer};
use tokio_util::sync::CancellationToken;
use tonic::transport::Server;

pub async fn serve(
    svc: impl ApiService + 'static,
    address: &str,
    shutdown: CancellationToken,
) -> Result<()> {
    let addr = address.parse()?;
    Server::builder()
        .add_service(ApiServiceServer::new(svc))
        .serve_with_shutdown(addr, shutdown.cancelled_owned())
        .await
        .map_err(|e| anyhow::anyhow!("gRPC server failed: {e}"))
}
