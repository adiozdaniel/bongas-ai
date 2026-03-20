//! Feature repository with Netflix-grade resilience patterns.
//!
//! Provides database access for user and item features with:
//! - Circuit breaker protection against cascading failures
//! - Bulkhead pattern for concurrency limiting
//! - Comprehensive error classification and metrics

use std::sync::Arc;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::{ItemFeatures, ProfileFeatures, VisitorFeatures};
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

    /// Get profile features by profile ID.
    ///
    /// Executes through circuit breaker with bulkhead protection.
    pub async fn get_profile_features(&self, profile_id: &str) -> AppResult<Option<ProfileFeatures>> {
        let profile_id = profile_id.to_string();
        let profile_log = profile_id.clone();
        let start_time = std::time::Instant::now();
        
        let result = self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ProfileFeatures>(
                    "SELECT * FROM bongas.profile_features WHERE profile_id = $1",
                )
                .bind(profile_id)
                .fetch_optional(&pool)
                .await
            })
            .await;

        let duration = start_time.elapsed();
        
        match result {
            Ok(features) => {
                // Record successful operation
                let metrics = self.metrics_collector.registry().get_or_create("feature_profile");
                metrics.latency.record_duration(duration);
                metrics.successes.increment();
                Ok(features)
            }
            Err(e) => {
                // Record failed operation
                let metrics = self.metrics_collector.registry().get_or_create("feature_profile");
                metrics.latency.record_duration(duration);
                metrics.failures.increment();
                
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch profile features for profile_id={}: {}", profile_log, e),
                    source: None,
                }))
            }
        }
    }

    /// Get visitor features by visitor ID.
    pub async fn get_visitor_features(&self, visitor_id: &str) -> AppResult<Option<VisitorFeatures>> {
        let visitor_id = visitor_id.to_string();
        let start_time = std::time::Instant::now();
        
        let result = self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, VisitorFeatures>(
                    "SELECT * FROM bongas.visitor_features WHERE visitor_id = $1",
                )
                .bind(visitor_id)
                .fetch_optional(&pool)
                .await
            })
            .await;

        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create("feature_visitor");
        metrics.latency.record_duration(duration);

        match result {
            Ok(features) => {
                metrics.successes.increment();
                Ok(features)
            }
            Err(e) => {
                metrics.failures.increment();
                Err(AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to fetch visitor features: {}", e),
                    source: None,
                }))
            }
        }
    }

    /// Get item features by item ID.
    pub async fn get_item_features(&self, item_id: i32) -> AppResult<Option<ItemFeatures>> {
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatures>(
                    "SELECT * FROM bongas.item_features WHERE item_id = $1",
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
                    "SELECT * FROM bongas.item_features ORDER BY trending_score DESC LIMIT $1",
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

    /// Batch get profile features for multiple profile IDs.
    pub async fn get_profile_features_batch(&self, profile_ids: &[String]) -> AppResult<Vec<ProfileFeatures>> {
        let profile_ids = profile_ids.to_vec();
        self.pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ProfileFeatures>(
                    "SELECT * FROM bongas.profile_features WHERE profile_id = ANY($1)",
                )
                .bind(&profile_ids)
                .fetch_all(&pool)
                .await
            })
            .await
            .map_err(|e| {
                AppError::Postgres(PostgresError::Query {
                    message: format!("Failed to batch fetch profile features: {}", e),
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
                    "SELECT * FROM bongas.item_features WHERE item_id = ANY($1)",
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

    /// Merge visitor features into profile features.
    pub async fn merge_visitor_features(&self, visitor_id: &str, user_id: i32, profile_id: &str) -> AppResult<()> {
        let vid = visitor_id.to_string();
        let pid = profile_id.to_string();
        
        self.pool.execute(move |pool| async move {
            sqlx::query(
                r#"
                INSERT INTO bongas.profile_features (profile_id, user_id, genre_affinity, total_watch_time_minutes, features_updated_at)
                SELECT $1, $2, genre_affinity, total_watch_time_minutes, NOW()
                FROM bongas.visitor_features
                WHERE visitor_id = $3
                ON CONFLICT (profile_id) DO UPDATE SET
                    genre_affinity = bongas.profile_features.genre_affinity || EXCLUDED.genre_affinity,
                    total_watch_time_minutes = bongas.profile_features.total_watch_time_minutes + EXCLUDED.total_watch_time_minutes,
                    features_updated_at = NOW()
                "#
            )
            .bind(pid)
            .bind(user_id)
            .bind(vid)
            .execute(&pool)
            .await?;
            
            Ok(())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query {
            message: format!("Failed to merge visitor features: {}", e),
            source: None,
        }))
    }
}
