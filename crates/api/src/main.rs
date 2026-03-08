mod api;
mod config;
mod grpc;
mod handlers;
mod repositories;
mod services;

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
