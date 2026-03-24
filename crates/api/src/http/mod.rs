use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;

pub async fn serve(router: Router, address: &str) -> Result<()> {
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
