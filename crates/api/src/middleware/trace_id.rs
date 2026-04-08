use std::convert::Infallible;

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, Response},
    middleware::Next,
};

use crate::http::trace_id::TraceId;

const TRACE_ID_HEADER: &str = "x-trace-id";

pub async fn mw_trace_id(mut req: Request<Body>, next: Next) -> Result<Response<Body>, Infallible> {
    let trace_id = req
        .headers()
        .get(TRACE_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.parse::<TraceId>().ok())
        .unwrap_or_else(TraceId::new);

    req.extensions_mut().insert(trace_id);

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        TRACE_ID_HEADER,
        HeaderValue::from_str(&trace_id.to_string()).unwrap(),
    );

    Ok(response)
}
