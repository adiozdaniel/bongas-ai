use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Response, IntoResponse},
    body::Body,
};
use serde_json::json;
use tracing::error;
use chrono::Utc;

/// Error handling middleware that provides consistent error responses
pub async fn error_handling_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let response = next.run(req).await;

    // If error response, enhance error body
    if response.status().is_client_error() || response.status().is_server_error() {
        let status = response.status();
        let message = match status {
            StatusCode::NOT_FOUND => "Resource not found",
            StatusCode::BAD_REQUEST => "Bad request",
            StatusCode::UNAUTHORIZED => "Unauthorized",
            StatusCode::FORBIDDEN => "Forbidden",
            StatusCode::TOO_MANY_REQUESTS => "Too many requests",
            StatusCode::INTERNAL_SERVER_ERROR => "Internal server error",
            StatusCode::SERVICE_UNAVAILABLE => "Service temporarily unavailable",
            StatusCode::GATEWAY_TIMEOUT => "Gateway timeout",
            _ => "Unknown error",
        };

        error!(
            status_code = %status,
            method = %method,
            uri = %uri,
            message = message,
            "Error response generated"
        );

        let error_response = json!({
            "success": false,
            "error": message,
            "status_code": status.as_u16(),
            "method": method.to_string(),
            "path": uri.path(),
            "timestamp": Utc::now().to_rfc3339(),
        });

        return (
            status,
            Body::from(error_response.to_string()),
        )
        .into_response();
    }

    response
}

/// Enhanced error handling with detailed error information
pub struct EnhancedErrorMiddleware;

impl EnhancedErrorMiddleware {
    pub async fn layer(
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let start_time = std::time::Instant::now();

        let response = next.run(req).await;
        let status = response.status();

        if status.is_client_error() || status.is_server_error() {
            let error_details = Self::build_error_details(&method, &uri, status, start_time.elapsed());
            
            error!(
                status_code = %status,
                method = %method,
                uri = %uri,
                error_details = ?error_details,
                "Enhanced error response"
            );

            let error_response = json!({
                "success": false,
                "error": {
                    "message": error_details.message,
                    "code": error_details.code,
                    "details": error_details.details,
                },
                "status_code": status.as_u16(),
                "method": method.to_string(),
                "path": uri.path(),
                "timestamp": Utc::now().to_rfc3339(),
                "duration_ms": start_time.elapsed().as_millis(),
            });

            return (
                status,
                Body::from(error_response.to_string()),
            )
            .into_response();
        }

        response
    }

    fn build_error_details(
        method: &axum::http::Method,
        uri: &axum::http::Uri,
        status: StatusCode,
        duration: std::time::Duration,
    ) -> ErrorDetails {
        let (message, code, details) = match status {
            StatusCode::NOT_FOUND => (
                "The requested resource was not found".to_string(),
                "RESOURCE_NOT_FOUND".to_string(),
                format!("No resource found at {} {}", method, uri.path()),
            ),
            StatusCode::BAD_REQUEST => (
                "The request was invalid or malformed".to_string(),
                "INVALID_REQUEST".to_string(),
                "Please check your request parameters and try again".to_string(),
            ),
            StatusCode::UNAUTHORIZED => (
                "Authentication is required to access this resource".to_string(),
                "UNAUTHORIZED".to_string(),
                "Please provide valid authentication credentials".to_string(),
            ),
            StatusCode::FORBIDDEN => (
                "You do not have permission to access this resource".to_string(),
                "FORBIDDEN".to_string(),
                "Your credentials are valid but insufficient for this resource".to_string(),
            ),
            StatusCode::TOO_MANY_REQUESTS => (
                "Rate limit exceeded".to_string(),
                "RATE_LIMIT_EXCEEDED".to_string(),
                format!("Too many requests. Please wait {} seconds before trying again", 60),
            ),
            StatusCode::INTERNAL_SERVER_ERROR => (
                "An internal server error occurred".to_string(),
                "INTERNAL_ERROR".to_string(),
                "Please try again later or contact support if the problem persists".to_string(),
            ),
            StatusCode::SERVICE_UNAVAILABLE => (
                "The service is temporarily unavailable".to_string(),
                "SERVICE_UNAVAILABLE".to_string(),
                "Please try again later".to_string(),
            ),
            StatusCode::GATEWAY_TIMEOUT => (
                "The server did not respond in time".to_string(),
                "GATEWAY_TIMEOUT".to_string(),
                "The upstream server timed out. Please try again".to_string(),
            ),
            _ => (
                "An unexpected error occurred".to_string(),
                "UNKNOWN_ERROR".to_string(),
                "Please try again or contact support".to_string(),
            ),
        };

        ErrorDetails {
            message,
            code,
            details,
            duration_ms: duration.as_millis() as u64,
        }
    }
}

#[derive(Debug)]
struct ErrorDetails {
    message: String,
    code: String,
    details: String,
    duration_ms: u64,
}

/// Validation error middleware for API validation errors
pub async fn validation_error_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let response = next.run(req).await;

    // Check if this is a validation error (422 Unprocessable Entity)
    if response.status() == StatusCode::UNPROCESSABLE_ENTITY {
        let error_response = json!({
            "success": false,
            "error": "Validation failed",
            "message": "The request data failed validation",
            "status_code": 422,
            "timestamp": Utc::now().to_rfc3339(),
        });

        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Body::from(error_response.to_string()),
        )
        .into_response();
    }

    response
}