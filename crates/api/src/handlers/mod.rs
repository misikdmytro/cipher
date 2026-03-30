mod common;
mod secrets;
mod webhooks;

use std::sync::Arc;

use axum::{Router, middleware::from_fn};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Cipher API",
        description = "HTTP API for creating and managing secret rotation workflows.",
        version = "0.1.0",
        contact(
            name = "Cipher Team",
            email = "cipher@example.com"
        ),
        license(
            name = "Proprietary"
        )
    ),
    servers(
        (url = "http://localhost:3000", description = "Local development")
    ),
    tags(
        (name = "secrets", description = "Create and manage secret rotation entries"),
        (name = "webhooks", description = "Register webhook notifications for rotation events")
    )
)]
struct ApiDoc;

pub fn router(state: Arc<AppState>) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(secrets::save_secret, secrets::list_secrets))
        .routes(routes!(secrets::get_secret_by_id))
        .routes(routes!(webhooks::register_webhook))
        .with_state(state)
        .split_for_parts();

    router
        .merge(::common::health::health_router())
        .layer(from_fn(crate::middleware::mw_log_traffic))
        .layer(from_fn(crate::middleware::mw_trace_id))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
}
