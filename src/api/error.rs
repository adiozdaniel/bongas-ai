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

    #[error("ONNX runtime error: {0}")]
    Onnx(String),

    #[error("Scenario not found: {slug}")]
    ScenarioNotFound { slug: String },

    #[error("Invalid pipeline configuration")]
    InvalidPipeline,

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ScenarioNotFound { slug } => {
                (StatusCode::NOT_FOUND, format!("Scenario '{}' not found", slug))
            }
            ApiError::InvalidPipeline => {
                (StatusCode::BAD_REQUEST, "Invalid pipeline configuration".to_string())
            }
            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg)
            }
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            }
            ApiError::Forbidden => {
                (StatusCode::FORBIDDEN, "Forbidden".to_string())
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
            ApiError::Onnx(e) => {
                error!(error = ?e, "ONNX error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Model inference error".to_string())
            }
            ApiError::Cache(msg) => {
                error!(error = %msg, "Cache error");
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
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

// Type alias for Result with ApiError
pub type ApiResult<T> = Result<T, ApiError>;