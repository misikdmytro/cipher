use std::{convert::Infallible, time::Instant};

use axum::{body::Body, extract::Request, http::Response, middleware::Next};

use crate::http::trace_id::TraceId;

pub async fn mw_log_traffic(req: Request<Body>, next: Next) -> Result<Response<Body>, Infallible> {
    let started_at = Instant::now();

    let trace_id = req.extensions().get::<TraceId>().copied();
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    tracing::debug!(trace_id = ?trace_id, method = %method, path = %path, "Received request");

    let response = next.run(req).await;

    let status = response.status();
    let duration = started_at.elapsed();

    tracing::debug!(trace_id = ?trace_id, method = %method, path = %path, status = %status, duration = ?duration, "Request completed");

    Ok(response)
}
