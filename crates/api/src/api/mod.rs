use std::sync::Arc;

use anyhow::Result;
use tokio::net::TcpListener;

use crate::{handlers, state::AppState};

pub async fn serve(state: Arc<AppState>) -> Result<()> {
    let address = state.config.api.address_without_scheme();
    let router = handlers::router(state);

    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
