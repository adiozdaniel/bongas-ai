//! Interaction repository with Netflix-grade resilience patterns and arrival tracking.

use std::sync::Arc;
use chrono::Timelike;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Repository for user interaction data with resilience patterns.
pub struct InteractionRepository {
    pool: Arc<ResilientPool>,
    metrics_collector: Arc<ResilienceMetricsCollector>,
}

impl InteractionRepository {
    pub fn new(
        pool: Arc<ResilientPool>,
        metrics_collector: Arc<ResilienceMetricsCollector>,
    ) -> Self {
        Self {
            pool,
            metrics_collector,
        }
    }

    pub async fn create_implicit_rating(
        &self,
        user_id: i32,
        item_id: i32,
        rating: f32,
        watch_duration_seconds: i32,
        visitor_id: Option<String>,
        device_hash: Option<String>,
        device_type: Option<String>,
    ) -> AppResult<()> {
        let start_time = std::time::Instant::now();
        let result = self.pool.execute(|pool| async move {
            sqlx::query(
                r#"
                INSERT INTO user_interactions
                    (user_id, item_id, interaction_type, rating, watch_duration_seconds, visitor_id, device_hash, device_type, created_at)
                VALUES ($1, $2, 'implicit_rating', $3, $4, $5, $6, $7, NOW())
                "#,
            )
            .bind(user_id)
            .bind(item_id)
            .bind(rating)
            .bind(watch_duration_seconds)
            .bind(visitor_id)
            .bind(device_hash)
            .bind(device_type)
            .execute(&pool)
            .await
            .map(|_| ())
        }).await;

        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create("interaction_implicit");
        metrics.latency.record_duration(duration);
        if result.is_ok() { metrics.successes.increment(); } else { metrics.failures.increment(); }

        result.map_err(|e| AppError::Postgres(PostgresError::Query {
            message: format!("Failed to create implicit rating: {}", e),
            source: None,
        }))
    }

    pub async fn create_interactions_batch(
        &self,
        user_ids: Vec<i32>,
        item_ids: Vec<i32>,
        types: Vec<String>,
        ratings: Vec<Option<f32>>,
        watch_durations: Vec<Option<i32>>,
        visitor_ids: Vec<Option<String>>,
        device_hashes: Vec<Option<String>>,
        device_types: Vec<Option<String>>,
        timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<u64> {
        if user_ids.is_empty() { return Ok(0); }
        self.pool.execute(|pool| async move {
            sqlx::query(
                r#"
                INSERT INTO user_interactions (user_id, item_id, interaction_type, rating, watch_duration_seconds, visitor_id, device_hash, device_type, created_at)
                SELECT * FROM unnest($1::int[], $2::int[], $3::text[], $4::float4[], $5::int[], $6::text[], $7::text[], $8::text[], $9::timestamptz[])
                "#
            )
            .bind(&user_ids).bind(&item_ids).bind(&types).bind(&ratings).bind(&watch_durations)
            .bind(&visitor_ids).bind(&device_hashes).bind(&device_types)
            .bind(&timestamps)
            .execute(&pool).await.map(|r| r.rows_affected())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }

    pub async fn update_arrival_pattern(&self, user_id: i32) -> AppResult<()> {
        let hour = chrono::Utc::now().hour() as i32;
        let bit_mask = 1i64 << hour;
        self.pool.execute(|pool| async move {
            sqlx::query(
                r#"
                INSERT INTO user_arrival_patterns (user_id, hour_mask, last_active_at)
                VALUES ($1, $2, NOW())
                ON CONFLICT (user_id) DO UPDATE SET hour_mask = user_arrival_patterns.hour_mask | $2, last_active_at = NOW()
                "#
            ).bind(user_id).bind(bit_mask).execute(&pool).await.map(|_| ())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }

    pub async fn get_likely_arrivals(&self, target_hour: u32) -> AppResult<Vec<i32>> {
        let bit_mask = 1i64 << target_hour;
        self.pool.execute(|pool| async move {
            sqlx::query_as::<_, (i32,)>("SELECT user_id FROM user_arrival_patterns WHERE (hour_mask & $1) != 0")
            .bind(bit_mask).fetch_all(&pool).await.map(|rows| rows.into_iter().map(|r| r.0).collect())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }

    /// Get top engaged users in the last 30 days (High priority for cache warming)
    pub async fn get_top_engaged_users(&self, limit: i32) -> AppResult<Vec<i32>> {
        self.pool.execute(|pool| async move {
            sqlx::query_as::<_, (i32,)> (
                r#"
                SELECT user_id
                FROM user_interactions
                WHERE created_at > NOW() - INTERVAL '30 days'
                GROUP BY user_id
                ORDER BY COUNT(*) DESC
                LIMIT $1
                "#
            )
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map(|rows| rows.into_iter().map(|r| r.0).collect())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }
}
