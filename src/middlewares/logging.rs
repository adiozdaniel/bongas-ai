use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    body::Body,
};
use tracing::{info, warn, error};
use uuid::Uuid;
use std::time::Instant;

/// Request logging middleware with correlation IDs
pub async fn logging_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let request_id = Uuid::new_v4().to_string();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let user_agent = req.headers()
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    // Add request ID to headers
    req.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        uri = %uri,
        version = ?version,
        user_agent = %user_agent,
    );

    let _enter = span.enter();

    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        user_agent = %user_agent,
        "Request started"
    );

    let start = Instant::now();
    let response = next.run(req).await;
    let latency = start.elapsed();

    let status = response.status();
    let latency_ms = latency.as_millis();

    if status.is_success() {
        info!(
            request_id = %request_id,
            status = %status,
            latency_ms = %latency_ms,
            "Request completed successfully"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            status = %status,
            latency_ms = %latency_ms,
            "Request completed with client error"
        );
    } else {
        error!(
            request_id = %request_id,
            status = %status,
            latency_ms = %latency_ms,
            "Request completed with server error"
        );
    }

    // Add request ID to response headers
    let mut response = response;
    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    response
}

