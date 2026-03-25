pub mod notificator_service;

use anyhow::Result;
use proto::notificator::notificator_service_server::{
    NotificatorService, NotificatorServiceServer,
};
use tonic::transport::Server;

pub async fn serve(svc: impl NotificatorService + 'static, address: &str) -> Result<()> {
    let addr = address.parse()?;
    Server::builder()
        .add_service(NotificatorServiceServer::new(svc))
        .serve(addr)
        .await
        .map_err(|e| anyhow::anyhow!("gRPC server failed: {e}"))
}
