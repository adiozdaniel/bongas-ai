use std::time::Duration;
use std::sync::Arc;
use thiserror::Error;
use crate::error::classification::{ErrorClassification, ErrorClassifier};

#[derive(Debug, Error)]
pub enum RedisError {
    #[error("redis connection failed: {message}")]
    Connection { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
    #[error("redis serialization failed: {0}")]
    Serialization(String),
    #[error("redis pool exhausted")]
    PoolExhausted,
    #[error("redis operation timed out after {0:?}")]
    Timeout(Duration),
    #[error("redis command failed: {message}")]
    Command { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
}

impl ErrorClassifier for RedisError {
    fn classify(&self) -> ErrorClassification {
        match self {
            RedisError::Connection { .. } => ErrorClassification::Transient,
            RedisError::Serialization(_) => ErrorClassification::Permanent,
            RedisError::PoolExhausted => ErrorClassification::Overload,
            RedisError::Timeout(_) => ErrorClassification::Timeout,
            RedisError::Command { .. } => ErrorClassification::Transient,
        }
    }
}

impl From<redis::RedisError> for RedisError {
    fn from(err: redis::RedisError) -> Self {
        if err.is_timeout() {
            RedisError::Timeout(Duration::from_secs(30))
        } else if err.is_connection_refusal() || err.is_io_error() {
            RedisError::Connection {
                message: err.to_string(),
                source: Some(Box::new(err)),
            }
        } else {
            RedisError::Command {
                message: err.to_string(),
                source: Some(Box::new(err)),
            }
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum PostgresError {
    #[error("postgres query failed: {message}")]
    Query { message: String, #[source] source: Option<Arc<dyn std::error::Error + Send + Sync>> },
    #[error("postgres pool exhausted")]
    PoolExhausted,
    #[error("postgres operation timed out after {0:?}")]
    Timeout(Duration),
    #[error("postgres migration failed: {0}")]
    Migration(String),
    #[error("postgres connection failed: {message}")]
    Connection { message: String, #[source] source: Option<Arc<dyn std::error::Error + Send + Sync>> },
    #[error("postgres constraint violation: {0}")]
    ConstraintViolation(String),
    #[error("postgres circuit breaker is open")]
    CircuitOpen,
}

impl ErrorClassifier for PostgresError {
    fn classify(&self) -> ErrorClassification {
        match self {
            PostgresError::Query { .. } => ErrorClassification::Transient,
            PostgresError::PoolExhausted => ErrorClassification::Overload,
            PostgresError::Timeout(_) => ErrorClassification::Timeout,
            PostgresError::Migration(_) => ErrorClassification::Permanent,
            PostgresError::Connection { .. } => ErrorClassification::Transient,
            PostgresError::ConstraintViolation(_) => ErrorClassification::Permanent,
            PostgresError::CircuitOpen => ErrorClassification::Overload,
        }
    }
}

#[derive(Debug, Error)]
pub enum ClickHouseError {
    #[error("clickhouse query failed: {message}")]
    Query { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
    #[error("clickhouse connection failed: {message}")]
    Connection { message: String, #[source] source: Option<Box<dyn std::error::Error + Send + Sync>> },
    #[error("clickhouse operation timed out after {0:?}")]
    Timeout(Duration),
}

impl ErrorClassifier for ClickHouseError {
    fn classify(&self) -> ErrorClassification {
        match self {
            ClickHouseError::Query { .. } => ErrorClassification::Transient,
            ClickHouseError::Connection { .. } => ErrorClassification::Transient,
            ClickHouseError::Timeout(_) => ErrorClassification::Timeout,
        }
    }
}
