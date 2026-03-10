mod secrets;

use std::sync::Arc;

use axum::{Router, routing::post};

use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/secrets", post(secrets::save_secret))
        .with_state(state)
}
