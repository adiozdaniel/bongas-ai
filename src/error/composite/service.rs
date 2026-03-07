use thiserror::Error;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::error;
use crate::error::classification::{ErrorClassification, ErrorClassifier};
use crate::error::domain::{PostgresError, RedisError, ClickHouseError, ModelError, ScenarioError, SecurityError, MiddlewareError, MetricsError};
use crate::error::retry::RetryHint;

/// Composite error type for the entire application.
///
/// Implements IntoResponse for seamless integration with Axum and
/// ErrorClassifier for integration with resilience patterns.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Postgres(#[from] PostgresError),

    #[error("cache error: {0}")]
    Redis(#[from] RedisError),

    #[error("analytics storage error: {0}")]
    ClickHouse(#[from] ClickHouseError),

    #[error("model error: {0}")]
    Model(#[from] ModelError),

    #[error("scenario error: {0}")]
    Scenario(#[from] ScenarioError),

    #[error("security error: {0}")]
    Security(#[from] SecurityError),

    #[error("middleware error: {0}")]
    Middleware(#[from] MiddlewareError),

    #[error("metrics error: {0}")]
    Metrics(#[from] MetricsError),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("internal server error: {0}")]
    Internal(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl ErrorClassifier for AppError {
    fn classify(&self) -> ErrorClassification {
        match self {
            AppError::Postgres(e) => e.classify(),
            AppError::Redis(e) => e.classify(),
            AppError::ClickHouse(e) => e.classify(),
            AppError::Model(e) => e.classify(),
            AppError::Scenario(e) => e.classify(),
            AppError::Security(e) => e.classify(),
            AppError::Middleware(e) => e.classify(),
            AppError::Metrics(e) => e.classify(),
            AppError::Anyhow(_) => ErrorClassification::Permanent,
            AppError::Internal(_) => ErrorClassification::Permanent,
            AppError::NotFound(_) => ErrorClassification::Permanent,
            AppError::Unauthorized(_) => ErrorClassification::Permanent,
            AppError::Forbidden(_) => ErrorClassification::Permanent,
        }
    }

    fn retry_hint(&self) -> RetryHint {
        match self {
            AppError::Postgres(e) => e.retry_hint(),
            AppError::Redis(e) => e.retry_hint(),
            AppError::ClickHouse(e) => e.retry_hint(),
            AppError::Model(e) => e.retry_hint(),
            AppError::Scenario(e) => e.retry_hint(),
            AppError::Security(e) => e.retry_hint(),
            AppError::Middleware(e) => e.retry_hint(),
            AppError::Metrics(e) => e.retry_hint(),
            _ => RetryHint::no_retry(),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let classification = self.classify();
        let status = match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Security(_) => StatusCode::UNAUTHORIZED,
            _ => {
                match classification {
                    ErrorClassification::Overload => StatusCode::SERVICE_UNAVAILABLE,
                    ErrorClassification::Timeout => StatusCode::GATEWAY_TIMEOUT,
                    ErrorClassification::Permanent => StatusCode::BAD_REQUEST,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
        };

        // Standard response formatting
        let message = self.to_string();
        let error_code = match self {
            AppError::Postgres(_) => "DATABASE_ERROR",
            AppError::Redis(_) => "CACHE_ERROR",
            AppError::ClickHouse(_) => "ANALYTICS_ERROR",
            AppError::Model(_) => "MODEL_ERROR",
            AppError::Scenario(_) => "SCENARIO_ERROR",
            AppError::Security(_) => "SECURITY_ERROR",
            AppError::Middleware(_) => "MIDDLEWARE_ERROR",
            AppError::Metrics(_) => "METRICS_ERROR",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            _ => "INTERNAL_SERVER_ERROR",
        };

        let retry_hint = self.retry_hint();
        let retry_after = retry_hint.retry_after.map(|d| d.as_millis() as u64);

        error!(
            error = ?self,
            status = %status,
            error_code = error_code,
            classification = ?classification,
            "Application error occurred"
        );

        let mut response = crate::api::StandardResponse::<()>::error(
            message,
            error_code,
            format!("{:?}", classification),
            classification.is_retriable(),
        );

        if let Some(ms) = retry_after {
            response = response.with_retry_after(ms);
        }

        (status, Json(response)).into_response()
    }
}
