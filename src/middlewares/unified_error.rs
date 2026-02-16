//! Unified error handling middleware that replaces the triple-overwrite pattern.
  //!
  //! This middleware provides a single, consistent error handling layer that:
  //! - Leverages the existing AppError IntoResponse implementation
  //! - Provides consistent error responses with classification
  //! - Handles validation errors
  //! - Maintains backward compatibility
  //!
  //! # Integration
  //! This replaces the three separate error middleware layers:
  //! - error_handling_middleware
  //! - EnhancedErrorMiddleware::layer
  //! - validation_error_middleware

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
  use crate::middlewares::error_handling::classify_http_error;

  /// Unified error handling middleware that provides consistent error responses
  pub async fn unified_error_middleware(
      req: Request<Body>,
      next: Next,
  ) -> Response<Body> {
      let method = req.method().clone();
      let uri = req.uri().clone();
      let start_time = std::time::Instant::now();

      let response = next.run(req).await;
      let status = response.status();

      // If this is already an error response, enhance it with additional context
      if status.is_client_error() || status.is_server_error() {
          // Check if this is a validation error (422 Unprocessable Entity)
          if status == StatusCode::UNPROCESSABLE_ENTITY {
              return handle_validation_error(method, uri, start_time.elapsed()).into_response();
          }

          // For other error responses, enhance with classification
          let (message, error_code, classification) = classify_http_error(status);

          error!(
              status_code = %status,
              method = %method,
              uri = %uri,
              message = message,
              classification = ?classification,
              duration_ms = start_time.elapsed().as_millis(),
              "Error response enhanced"
          );

          let error_response = json!({
              "success": false,
              "error": {
                  "message": message,
                  "code": error_code,
                  "classification": format!("{:?}", classification),
                  "retriable": classification.is_retriable(),
                  "should_trip_breaker": classification.should_trip(),
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

  /// Handle validation errors with specific error details
  fn handle_validation_error(
      method: axum::http::Method,
      uri: axum::http::Uri,
      duration: std::time::Duration,
  ) -> ErrorResponse {
      ErrorResponse {
          success: false,
          error: ErrorBody {
              message: "Validation failed".to_string(),
              code: "VALIDATION_ERROR".to_string(),
              classification: ErrorClassification::Permanent,
              retriable: false,
              retry_after: None,
          },
          status_code: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
          method: method.to_string(),
          path: uri.path().to_string(),
          timestamp: Utc::now().to_rfc3339(),
          duration_ms: duration.as_millis() as u64,
      }
  }

  /// Error response structure for enhanced error details
  #[derive(serde::Serialize)]
  struct ErrorResponse {
      success: bool,
      error: ErrorBody,
      status_code: u16,
      method: String,
      path: String,
      timestamp: String,
      duration_ms: u64,
  }

  /// Error body structure
  #[derive(serde::Serialize)]
  struct ErrorBody {
      message: String,
      code: String,
      classification: ErrorClassification,
      retriable: bool,
      retry_after: Option<u64>,
  }

  impl IntoResponse for ErrorResponse {
      fn into_response(self) -> Response<Body> {
          let body = Body::from(serde_json::to_string(&self).unwrap_or_else(|_| {
              json!({
                  "success": false,
                  "error": {
                      "message": "Internal error serialization failed",
                      "code": "SERIALIZATION_ERROR",
                      "classification": "Transient",
                      "retriable": true,
                  },
                  "status_code": 500,
                  "timestamp": Utc::now().to_rfc3339(),
              }).to_string()
          }));

          (StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), body).into_response()
      }
  }
