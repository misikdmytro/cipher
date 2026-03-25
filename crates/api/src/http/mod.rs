use anyhow::Result;
use axum::Router;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

pub async fn serve(router: Router, address: &str, shutdown: CancellationToken) -> Result<()> {
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await?;
    Ok(())
}
