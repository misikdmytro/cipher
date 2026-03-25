use anyhow::Result;
use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

async fn health_check() -> Json<Value> {
    Json(json!({"status": "ok"}))
}

pub fn health_router() -> Router {
    Router::new().route("/health", get(health_check))
}

pub async fn serve_health(address: &str, shutdown: CancellationToken) -> Result<()> {
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, health_router())
        .with_graceful_shutdown(shutdown.cancelled_owned())
        .await?;
    Ok(())
}
