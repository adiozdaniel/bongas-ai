//! Model repository with Netflix-grade resilience patterns.
//!
//! Provides database access for ML model registry with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;

use crate::analytics::ResilienceMetricsCollector;
use crate::db::models::ModelRegistry;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Repository for ML model registry with resilience patterns.
pub struct ModelRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl ModelRepository {
    /// Create a new repository with resilient pool and metrics collector.
    pub fn new(
        pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            pool,
            metrics_collector,
        }
    }

    /// Get all deployed ONNX models.
    ///
    /// Executes through circuit breaker with bulkhead protection.
    pub async fn get_deployed_onnx_models(&self) -> AppResult<Vec<ModelRegistry>> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ModelRegistry>(
                    r#"
                    SELECT * FROM model_registry
                    WHERE status = 'deployed'
                    AND model_format = 'onnx'
                    ORDER BY created_at DESC
                    "#,
                )
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch deployed ONNX models: {}", e),
                    source: None,
                })
            })
    }

    /// Get ONNX model by name and version.
    pub async fn get_onnx_model(
        &self,
        model_name: &str,
        version: &str,
    ) -> AppResult<Option<ModelRegistry>> {
        let model_name = model_name.to_string();
        let version = version.to_string();
        let model_name_err = model_name.clone();
        let version_err = version.clone();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ModelRegistry>(
                    r#"
                    SELECT * FROM model_registry
                    WHERE model_name = $1
                    AND version = $2
                    AND model_format = 'onnx'
                    "#,
                )
                .bind(&model_name)
                .bind(&version)
                .fetch_optional(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!(
                        "Failed to fetch ONNX model {}@{}: {}",
                        model_name_err, version_err, e
                    ),
                    source: None,
                })
            })
    }

    /// Get latest deployed model by name.
    pub async fn get_latest_deployed(&self, model_name: &str) -> AppResult<Option<ModelRegistry>> {
        let model_name = model_name.to_string();
        let model_name_err = model_name.clone();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ModelRegistry>(
                    r#"
                    SELECT * FROM model_registry
                    WHERE model_name = $1
                    AND status = 'deployed'
                    ORDER BY created_at DESC
                    LIMIT 1
                    "#,
                )
                .bind(&model_name)
                .fetch_optional(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch latest deployed model {}: {}", model_name_err, e),
                    source: None,
                })
            })
    }

    /// Get all models by status.
    pub async fn get_by_status(&self, status: &str) -> AppResult<Vec<ModelRegistry>> {
        let status = status.to_string();
        let status_err = status.clone();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ModelRegistry>(
                    r#"
                    SELECT * FROM model_registry
                    WHERE status = $1
                    ORDER BY created_at DESC
                    "#,
                )
                .bind(&status)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch models by status {}: {}", status_err, e),
                    source: None,
                })
            })
    }
}
