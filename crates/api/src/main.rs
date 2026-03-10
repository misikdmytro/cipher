mod api;
mod config;
mod handlers;
mod repositories;
mod services;
mod state;

use anyhow::Result;
use proto::scheduler::scheduler_service_client::SchedulerServiceClient;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<()> {
    let endpoint = Endpoint::new("http://[::1]:50052")?; // TODO: implement config here
    let channel = endpoint.connect().await?;
    let scheduler_client = SchedulerServiceClient::new(channel);

    let api = tokio::spawn(api::serve());
    let (api,) = tokio::try_join!(api)?;

    api?;

    Ok(())
}
