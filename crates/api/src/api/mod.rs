use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;

use crate::{handlers, state::AppState};

pub async fn serve(state: Arc<AppState>) -> Result<()> {
    let app = handlers::router(state);

    // TODO: configure it
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
