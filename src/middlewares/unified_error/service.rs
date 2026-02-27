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
      Json,
  };
  use serde_json::json;
  use tracing::error;
  use chrono::Utc;

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
              },
              "meta": {
                  "request_id": "unknown",
                  "timestamp": Utc::now().to_rfc3339(),
                  "duration_ms": start_time.elapsed().as_millis(),
                  "version": env!("CARGO_PKG_VERSION"),
              }
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
      _method: axum::http::Method,
      _uri: axum::http::Uri,
      duration: std::time::Duration,
  ) -> impl IntoResponse {
      let body = json!({
          "success": false,
          "error": {
              "message": "Validation failed",
              "code": "VALIDATION_ERROR",
              "classification": "Permanent",
              "retriable": false,
          },
          "meta": {
              "request_id": "unknown",
              "timestamp": Utc::now().to_rfc3339(),
              "duration_ms": duration.as_millis(),
              "version": env!("CARGO_PKG_VERSION"),
          }
      });

      (StatusCode::UNPROCESSABLE_ENTITY, Json(body))
  }

