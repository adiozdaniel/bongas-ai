use thiserror::Error;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::error;
use crate::error::classification::{ErrorClassification, ErrorClassifier};
use crate::error::domain::*;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Redis(#[from] RedisError),
    #[error(transparent)]
    Postgres(#[from] PostgresError),
    #[error(transparent)]
    ClickHouse(#[from] ClickHouseError),
    #[error(transparent)]
    Ingestion(#[from] IngestionError),
    #[error(transparent)]
    Cache(#[from] CacheError),
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Scenario(#[from] ScenarioError),
    #[error(transparent)]
    Security(#[from] SecurityError),
    #[error(transparent)]
    Middleware(#[from] MiddlewareError),
    #[error(transparent)]
    Metrics(#[from] MetricsError),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
}

impl ErrorClassifier for AppError {
    fn classify(&self) -> ErrorClassification {
        match self {
            AppError::Redis(e) => e.classify(),
            AppError::Postgres(e) => e.classify(),
            AppError::ClickHouse(e) => e.classify(),
            AppError::Ingestion(e) => e.classify(),
            AppError::Cache(e) => e.classify(),
            AppError::Pipeline(e) => e.classify(),
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
}

pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let classification = self.classify();
        let (status, error_code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            
            AppError::Scenario(ScenarioError::NotFound(slug)) => {
                (StatusCode::NOT_FOUND, "SCENARIO_NOT_FOUND", format!("Scenario '{}' not found", slug))
            }
            
            AppError::Middleware(MiddlewareError::RateLimitExceeded) => {
                (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMIT_EXCEEDED", "Rate limit exceeded. Please slow down.".to_string())
            }

            AppError::Postgres(PostgresError::CircuitOpen) | AppError::Redis(RedisError::PoolExhausted) => {
                (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_OVERLOADED", "Internal system resources are currently overwhelmed".to_string())
            }

            _ => match classification {
                ErrorClassification::Permanent => (StatusCode::BAD_REQUEST, "CLIENT_ERROR", format!("{}", self)),
                ErrorClassification::Transient => (StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE", "Service is temporarily unavailable".to_string()),
                ErrorClassification::Timeout => (StatusCode::GATEWAY_TIMEOUT, "GATEWAY_TIMEOUT", "The request timed out".to_string()),
                ErrorClassification::Overload => (StatusCode::TOO_MANY_REQUESTS, "SYSTEM_OVERLOAD", "System is under heavy load".to_string()),
                ErrorClassification::Degraded | ErrorClassification::PartialFailure => (StatusCode::MULTI_STATUS, "PARTIAL_SUCCESS", "Operation completed with partial results".to_string()),
            }
        };

        error!(
            error = ?self,
            classification = ?classification,
            status_code = %status,
            error_code = error_code,
            "AppError converted to HTTP response"
        );

        let retry_hint = self.retry_hint();
        let retry_after = retry_hint.retry_after.map(|d| d.as_millis() as u64);

        // Use StandardResponse for consistent serialization
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
