use anyhow::Result;
use proto::ping::{
    PingRequest, PongResponse,
    ping_service_server::{PingService, PingServiceServer},
};
use tonic::{Request, Response, Status, transport::Server};

pub struct PingServiceImpl;

#[tonic::async_trait]
impl PingService for PingServiceImpl {
    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PongResponse>, Status> {
        Ok(Response::new(PongResponse {
            message: "Pong".to_string(),
        }))
    }
}

pub async fn serve() -> Result<()> {
    let addr = "0.0.0.0:50051".parse()?;
    Server::builder()
        .add_service(PingServiceServer::new(PingServiceImpl))
        .serve(addr)
        .await?;

    Ok(())
}
