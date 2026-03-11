mod common;
mod secrets;

use std::sync::Arc;

use axum::Router;
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
        (name = "secrets", description = "Create and manage secret rotation entries")
    )
)]
struct ApiDoc;

pub fn router(state: Arc<AppState>) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(secrets::save_secret))
        .with_state(state)
        .split_for_parts();

    router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
}
