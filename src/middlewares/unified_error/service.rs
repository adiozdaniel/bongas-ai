//! Unified error handling middleware that replaces the triple-overwrite pattern.
//!
//! This middleware provides a single, consistent error handling layer that:
//! - Leverages the existing AppError IntoResponse implementation
//! - Provides consistent error responses with classification
//! - Handles validation errors
//! - Maintains backward compatibility

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Response, IntoResponse},
    body::Body,
    Json,
};
use tracing::error;
use crate::error::classification::ErrorClassification;

/// Unified error handling middleware that provides consistent error responses
pub async fn unified_error_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
      let method = req.method().clone();
      let uri = req.uri().clone();
      let start_time = std::time::Instant::now();
      let request_id = crate::api::extract_request_id(&req);

      let response = next.run(req).await;
      let status = response.status();

    // If this is already an error response, enhance it with additional context
    if status.is_client_error() || status.is_server_error() {
        // Check if this is a validation error (422 Unprocessable Entity)
        if status == StatusCode::UNPROCESSABLE_ENTITY {
            return handle_validation_error(method, uri, start_time.elapsed(), request_id).into_response();
        }

        // For other error responses, enhance with classification
        let (message, error_code, classification) = classify_http_error(status);

        error!(
            request_id = %request_id,
            status_code = %status,
            method = %method,
            uri = %uri,
            message = message,
            classification = ?classification,
            duration_ms = start_time.elapsed().as_millis(),
            "Error response enhanced"
        );

        let error_response = crate::api::StandardResponse::<()>::error(
            message,
            error_code,
            format!("{:?}", classification),
            classification.is_retriable(),
        )
        .with_request_id(request_id)
        .with_duration(start_time.elapsed().as_millis() as u64);

        return (
            status,
            Json(error_response),
        )
        .into_response();
    }

    response
}

/// Handle validation errors with specific error details
fn handle_validation_error(
    _method: axum::http::Method,
    _uri: axum::http::Uri,
    duration: std::time::Duration,
    request_id: String,
) -> impl IntoResponse {
    let response = crate::api::StandardResponse::<()>::error(
        "Validation failed",
        "VALIDATION_ERROR",
        "Permanent",
        false,
    )
    .with_request_id(request_id)
    .with_duration(duration.as_millis() as u64);

    (StatusCode::UNPROCESSABLE_ENTITY, Json(response))
}

/// Classify HTTP status codes into Bongas-AI error domains
fn classify_http_error(status: StatusCode) -> (String, String, ErrorClassification) {
    match status {
        StatusCode::UNAUTHORIZED => (
            "Unauthorized access".to_string(),
            "UNAUTHORIZED".to_string(),
            ErrorClassification::Permanent,
        ),
        StatusCode::FORBIDDEN => (
            "Access forbidden".to_string(),
            "FORBIDDEN".to_string(),
            ErrorClassification::Permanent,
        ),
        StatusCode::NOT_FOUND => (
            "Resource not found".to_string(),
            "NOT_FOUND".to_string(),
            ErrorClassification::Permanent,
        ),
        StatusCode::TOO_MANY_REQUESTS => (
            "Rate limit exceeded".to_string(),
            "RATE_LIMIT_EXCEEDED".to_string(),
            ErrorClassification::Transient,
        ),
        StatusCode::SERVICE_UNAVAILABLE => (
            "Service unavailable".to_string(),
            "SERVICE_UNAVAILABLE".to_string(),
            ErrorClassification::Transient,
        ),
        StatusCode::GATEWAY_TIMEOUT => (
            "Gateway timeout".to_string(),
            "TIMEOUT".to_string(),
            ErrorClassification::Transient,
        ),
        _ => (
            "Internal server error".to_string(),
            "INTERNAL_SERVER_ERROR".to_string(),
            ErrorClassification::Permanent,
        ),
    }
}
