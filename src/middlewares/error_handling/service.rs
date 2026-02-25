
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

  use crate::error::ErrorClassification;

  /// Error handling middleware that provides consistent error responses with classification
  #[deprecated(note = "Use unified_error_middleware instead")]
  pub async fn error_handling_middleware(
      req: Request<Body>,
      next: Next,
  ) -> Response<Body> {
      let method = req.method().clone();
      let uri = req.uri().clone();
      let response = next.run(req).await;

      // If error response, enhance error body with classification
      if response.status().is_client_error() || response.status().is_server_error() {
          let status = response.status();
          let (message, error_code, classification) = classify_http_error(status);

          error!(
              status_code = %status,
              method = %method,
              uri = %uri,
              message = message,
              classification = ?classification,
              "Error response generated"
          );

          let error_response = json!({
              "success": false,
              "error": {
                  "message": message,
                  "code": error_code,
                  "classification": format!("{:?}", classification),
                  "retriable": classification.is_retriable(),
              },
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
      #[deprecated(note = "Use unified_error_middleware instead")]
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
                  classification = ?error_details.classification,
                  "Enhanced error response"
              );

              let error_response = json!({
                  "success": false,
                  "error": {
                      "message": error_details.message,
                      "code": error_details.code,
                      "details": error_details.details,
                      "classification": format!("{:?}", error_details.classification),
                      "retriable": error_details.classification.is_retriable(),
                      "should_trip_breaker": error_details.classification.should_trip(),
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
          _duration: std::time::Duration,
      ) -> ErrorDetails {
          let (message, code, details, classification) = match status {
              StatusCode::NOT_FOUND => (
                  "The requested resource was not found".to_string(),
                  "RESOURCE_NOT_FOUND".to_string(),
                  format!("No resource found at {} {}", method, uri.path()),
                  ErrorClassification::Permanent,
              ),
              StatusCode::BAD_REQUEST => (
                  "The request was invalid or malformed".to_string(),
                  "INVALID_REQUEST".to_string(),
                  "Please check your request parameters and try again".to_string(),
                  ErrorClassification::Permanent,
              ),
              StatusCode::UNAUTHORIZED => (
                  "Authentication is required to access this resource".to_string(),
                  "UNAUTHORIZED".to_string(),
                  "Please provide valid authentication credentials".to_string(),
                  ErrorClassification::Permanent,
              ),
              StatusCode::FORBIDDEN => (
                  "You do not have permission to access this resource".to_string(),
                  "FORBIDDEN".to_string(),
                  "Your credentials are valid but insufficient for this resource".to_string(),
                  ErrorClassification::Permanent,
              ),
              StatusCode::TOO_MANY_REQUESTS => (
                  "Rate limit exceeded".to_string(),
                  "RATE_LIMIT_EXCEEDED".to_string(),
                  "Too many requests. Please wait before trying again".to_string(),
                  ErrorClassification::Overload,
              ),
              StatusCode::INTERNAL_SERVER_ERROR => (
                  "An internal server error occurred".to_string(),
                  "INTERNAL_ERROR".to_string(),
                  "Please try again later or contact support if the problem persists".to_string(),
                  ErrorClassification::Transient,
              ),
              StatusCode::SERVICE_UNAVAILABLE => (
                  "The service is temporarily unavailable".to_string(),
                  "SERVICE_UNAVAILABLE".to_string(),
                  "Please try again later".to_string(),
                  ErrorClassification::Overload,
              ),
              StatusCode::GATEWAY_TIMEOUT => (
                  "The server did not respond in time".to_string(),
                  "GATEWAY_TIMEOUT".to_string(),
                  "The upstream server timed out. Please try again".to_string(),
                  ErrorClassification::Timeout,
              ),
              _ => (
                  "An unexpected error occurred".to_string(),
                  "UNKNOWN_ERROR".to_string(),
                  "Please try again or contact support".to_string(),
                  ErrorClassification::Transient,
              ),
          };

          ErrorDetails {
              message,
              code,
              details,
              classification,
          }
      }
  }

  #[derive(Debug)]
  struct ErrorDetails {
      message: String,
      code: String,
      details: String,
      classification: ErrorClassification,
  }

  /// Classify HTTP status code to ErrorClassification
  pub fn classify_http_error(status: StatusCode) -> (&'static str, &'static str, ErrorClassification) {
      match status {
          StatusCode::NOT_FOUND => ("Resource not found", "RESOURCE_NOT_FOUND", ErrorClassification::Permanent),
          StatusCode::BAD_REQUEST => ("Bad request", "BAD_REQUEST", ErrorClassification::Permanent),
          StatusCode::UNAUTHORIZED => ("Unauthorized", "UNAUTHORIZED", ErrorClassification::Permanent),
          StatusCode::FORBIDDEN => ("Forbidden", "FORBIDDEN", ErrorClassification::Permanent),
          StatusCode::TOO_MANY_REQUESTS => ("Too many requests", "RATE_LIMIT_EXCEEDED", ErrorClassification::Overload),
          StatusCode::INTERNAL_SERVER_ERROR => ("Internal server error", "INTERNAL_ERROR", ErrorClassification::Transient),
          StatusCode::SERVICE_UNAVAILABLE => ("Service temporarily unavailable", "SERVICE_UNAVAILABLE",
  ErrorClassification::Overload),
          StatusCode::GATEWAY_TIMEOUT => ("Gateway timeout", "GATEWAY_TIMEOUT", ErrorClassification::Timeout),
          StatusCode::REQUEST_TIMEOUT => ("Request timeout", "REQUEST_TIMEOUT", ErrorClassification::Timeout),
          _ if status.is_client_error() => ("Client error", "CLIENT_ERROR", ErrorClassification::Permanent),
          _ if status.is_server_error() => ("Server error", "SERVER_ERROR", ErrorClassification::Transient),
          _ => ("Unknown error", "UNKNOWN_ERROR", ErrorClassification::Transient),
      }
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
              "error": {
                  "message": "Validation failed",
                  "code": "VALIDATION_ERROR",
                  "details": "The request data failed validation",
                  "classification": "Permanent",
                  "retriable": false,
              },
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

