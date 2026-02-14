//! Interaction repository with Netflix-grade resilience patterns.
//!
//! Provides database access for user interactions with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Repository for user interaction data with resilience patterns.
pub struct InteractionRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl InteractionRepository {
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

    /// Create an implicit rating from watch behavior.
    ///
    /// Executes through circuit breaker with bulkhead protection.
    pub async fn create_implicit_rating(
        &self,
        user_id: i32,
        item_id: i32,
        rating: f32,
        watch_duration_seconds: i32,
    ) -> AppResult<()> {
        self.pool
            .execute(|pool| async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_interactions
                        (user_id, item_id, interaction_type, rating, watch_duration_seconds, created_at)
                    VALUES ($1, $2, 'implicit_rating', $3, $4, NOW())
                    "#,
                )
                .bind(user_id)
                .bind(item_id)
                .bind(rating)
                .bind(watch_duration_seconds)
                .execute(&pool)
                .await
                .map(|_| ())
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!(
                        "Failed to create implicit rating for user={}, item={}: {}",
                        user_id, item_id, e
                    ),
                    source: None,
                })
            })
    }

    /// Create an explicit rating from user input.
    pub async fn create_explicit_rating(
        &self,
        user_id: i32,
        item_id: i32,
        rating: f32,
    ) -> AppResult<()> {
        self.pool
            .execute(|pool| async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_interactions
                        (user_id, item_id, interaction_type, rating, created_at)
                    VALUES ($1, $2, 'explicit_rating', $3, NOW())
                    "#,
                )
                .bind(user_id)
                .bind(item_id)
                .bind(rating)
                .execute(&pool)
                .await
                .map(|_| ())
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!(
                        "Failed to create explicit rating for user={}, item={}: {}",
                        user_id, item_id, e
                    ),
                    source: None,
                })
            })
    }

    /// Record a click interaction.
    pub async fn record_click(&self, user_id: i32, item_id: i32) -> AppResult<()> {
        self.pool
            .execute(|pool| async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_interactions
                        (user_id, item_id, interaction_type, created_at)
                    VALUES ($1, $2, 'click', NOW())
                    "#,
                )
                .bind(user_id)
                .bind(item_id)
                .execute(&pool)
                .await
                .map(|_| ())
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!(
                        "Failed to record click for user={}, item={}: {}",
                        user_id, item_id, e
                    ),
                    source: None,
                })
            })
    }

    /// Record an impression (item shown to user).
    pub async fn record_impression(&self, user_id: i32, item_id: i32) -> AppResult<()> {
        self.pool
            .execute(|pool| async move {
                sqlx::query(
                    r#"
                    INSERT INTO user_interactions
                        (user_id, item_id, interaction_type, created_at)
                    VALUES ($1, $2, 'impression', NOW())
                    "#,
                )
                .bind(user_id)
                .bind(item_id)
                .execute(&pool)
                .await
                .map(|_| ())
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!(
                        "Failed to record impression for user={}, item={}: {}",
                        user_id, item_id, e
                    ),
                    source: None,
                })
            })
    }

    /// Batch record impressions for multiple items.
    pub async fn record_impressions_batch(
        &self,
        user_id: i32,
        item_ids: &[i32],
    ) -> AppResult<u64> {
        let item_ids = item_ids.to_vec();
        self.pool
            .execute(|pool| async move {
                // Use unnest for efficient batch insert
                sqlx::query(
                    r#"
                    INSERT INTO user_interactions (user_id, item_id, interaction_type, created_at)
                    SELECT $1, unnest($2::int[]), 'impression', NOW()
                    "#,
                )
                .bind(user_id)
                .bind(&item_ids)
                .execute(&pool)
                .await
                .map(|r| r.rows_affected())
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!(
                        "Failed to batch record impressions for user={}: {}",
                        user_id, e
                    ),
                    source: None,
                })
            })
    }
}
