use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("ClickHouse error: {0}")]
    ClickHouse(#[from] clickhouse::error::Error),

    #[error("Scenario not found: {slug}")]
    ScenarioNotFound { slug: String },

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ScenarioNotFound { slug } => {
                (StatusCode::NOT_FOUND, format!("Scenario '{}' not found", slug))
            }
            ApiError::Database(e) => {
                error!(error = ?e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            ApiError::Redis(e) => {
                error!(error = ?e, "Redis error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Cache error".to_string())
            }
            ApiError::ClickHouse(e) => {
                error!(error = ?e, "ClickHouse error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Analytics error".to_string())
            }
            ApiError::Internal(msg) => {
                error!(error = %msg, "Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
        };

        let body = Json(json!({
            "success": false,
            "error": error_message,
            "timestamp": chrono::Utc::now()
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
