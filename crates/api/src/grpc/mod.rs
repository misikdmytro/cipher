pub mod api_service;

use anyhow::Result;
use proto::api::api_service_server::{ApiService, ApiServiceServer};
use tonic::transport::Server;

pub async fn serve(svc: impl ApiService + 'static, address: &str) -> Result<()> {
    let addr = address.parse()?;
    Server::builder()
        .add_service(ApiServiceServer::new(svc))
        .serve(addr)
        .await
        .map_err(|e| anyhow::anyhow!("gRPC server failed: {e}"))
}
