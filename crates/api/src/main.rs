mod api;
mod grpc;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let api = tokio::spawn(api::serve());
    let grpc = tokio::spawn(grpc::serve());
    let (api, grpc) = tokio::try_join!(api, grpc)?;

    api?;
    grpc?;

    Ok(())
}
