//! Feature repository with Netflix-grade resilience patterns.
//!
//! Provides database access for user and item features with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::models::{ItemFeatures, UserFeatures};
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Repository for feature data with resilience patterns.
pub struct FeatureRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl FeatureRepository {
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

    /// Get user features by user ID.
    ///
    /// Executes through circuit breaker with bulkhead protection.
    pub async fn get_user_features(&self, user_id: i32) -> AppResult<Option<UserFeatures>> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserFeatures>(
                    "SELECT * FROM user_features WHERE user_id = $1",
                )
                .bind(user_id)
                .fetch_optional(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch user features for user_id={}: {}", user_id, e),
                    source: None,
                })
            })
    }

    /// Get item features by item ID.
    pub async fn get_item_features(&self, item_id: i32) -> AppResult<Option<ItemFeatures>> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatures>(
                    "SELECT * FROM item_features WHERE item_id = $1",
                )
                .bind(item_id)
                .fetch_optional(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch item features for item_id={}: {}", item_id, e),
                    source: None,
                })
            })
    }

    /// Get trending items with limit.
    pub async fn get_trending_items(&self, limit: i64) -> AppResult<Vec<ItemFeatures>> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatures>(
                    "SELECT * FROM item_features ORDER BY trending_score DESC LIMIT $1",
                )
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch trending items: {}", e),
                    source: None,
                })
            })
    }

    /// Batch get user features for multiple user IDs.
    pub async fn get_user_features_batch(&self, user_ids: &[i32]) -> AppResult<Vec<UserFeatures>> {
        let user_ids = user_ids.to_vec();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserFeatures>(
                    "SELECT * FROM user_features WHERE user_id = ANY($1)",
                )
                .bind(&user_ids)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to batch fetch user features: {}", e),
                    source: None,
                })
            })
    }

    /// Batch get item features for multiple item IDs.
    pub async fn get_item_features_batch(&self, item_ids: &[i32]) -> AppResult<Vec<ItemFeatures>> {
        let item_ids = item_ids.to_vec();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatures>(
                    "SELECT * FROM item_features WHERE item_id = ANY($1)",
                )
                .bind(&item_ids)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to batch fetch item features: {}", e),
                    source: None,
                })
            })
    }
}
