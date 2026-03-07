use anyhow::Result;
use proto::ping::{PingRequest, ping_service_client::PingServiceClient};

pub async fn ping(addr: &str) -> Result<String> {
    let mut client = PingServiceClient::connect(addr.to_string()).await?;

    let response = client
        .ping(PingRequest {
            message: "hello from rotator".to_string(),
        })
        .await?;

    Ok(response.into_inner().message)
}
