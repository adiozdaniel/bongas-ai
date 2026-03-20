//! Interaction repository with Netflix-grade resilience patterns and arrival tracking.

use std::sync::Arc;
use chrono::Timelike;

use crate::resilience::ResilienceMetricsCollector;
use crate::db::ResilientPool;
use crate::error::{AppError, AppResult, PostgresError};

/// Payload for recording a single user interaction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InteractionPayload {
    pub user_id: i32,
    pub profile_id: Option<String>,
    pub item_id: i32,
    pub interaction_type: String,
    pub scenario_slug: String,
    pub weight: f32,
    pub visitor_id: Option<String>,
    pub device_hash: Option<String>,
    pub device_type: Option<String>,
    pub watch_duration_seconds: Option<i32>,
}

/// Payload for recording a batch of user interactions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchInteractionPayload {
    pub user_ids: Vec<i32>,
    pub item_ids: Vec<i32>,
    pub types: Vec<String>,
    pub ratings: Vec<Option<f32>>,
    pub watch_durations: Vec<Option<i32>>,
    pub visitor_ids: Vec<Option<String>>,
    pub device_hashes: Vec<Option<String>>,
    pub device_types: Vec<Option<String>>,
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
}

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

    /// Primary entry point for recording any user interaction.
    pub async fn record_interaction(
        &self,
        payload: InteractionPayload,
    ) -> AppResult<()> {
        let start_time = std::time::Instant::now();
        let i_type_for_metrics = payload.interaction_type.clone();
        
        let result = self.pool.execute(move |pool| {
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO bongas.user_interactions
                        (user_id, profile_id, item_id, interaction_type, implicit_rating, scenario_slug, visitor_id, device_hash, device_type, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
                    "#,
                )
                .bind(payload.user_id)
                .bind(payload.profile_id)
                .bind(payload.item_id)
                .bind(payload.interaction_type)
                .bind(payload.weight)
                .bind(payload.scenario_slug)
                .bind(payload.visitor_id)
                .bind(payload.device_hash)
                .bind(payload.device_type)
                .execute(&pool)
                .await
                .map(|_| ())
            }
        }).await;

        let duration = start_time.elapsed();
        let metrics = self.metrics_collector.registry().get_or_create(&format!("interaction_{}", i_type_for_metrics));
        metrics.latency.record_duration(duration);
        if result.is_ok() { metrics.successes.increment(); } else { metrics.failures.increment(); }

        result.map_err(|e| AppError::Postgres(PostgresError::Query {
            message: format!("Failed to record interaction: {}", e),
            source: None,
        }))
    }

    pub async fn create_implicit_rating(
        &self,
        payload: InteractionPayload,
    ) -> AppResult<()> {
        let start_time = std::time::Instant::now();
        let result = self.pool.execute(move |pool| {
            async move {
                sqlx::query(
                    r#"
                    INSERT INTO bongas.user_interactions
                        (user_id, profile_id, item_id, interaction_type, implicit_rating, watch_duration_seconds, scenario_slug, visitor_id, device_hash, device_type, created_at)
                    VALUES ($1, $2, $3, 'implicit_rating', $4, $5, $6, $7, $8, $9, NOW())
                    "#,
                )
                .bind(payload.user_id)
                .bind(payload.profile_id)
                .bind(payload.item_id)
                .bind(payload.weight)
                .bind(payload.watch_duration_seconds)
                .bind(payload.scenario_slug)
                .bind(payload.visitor_id)
                .bind(payload.device_hash)
                .bind(payload.device_type)
                .execute(&pool)
                .await
                .map(|_| ())
            }
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
        payload: BatchInteractionPayload,
    ) -> AppResult<u64> {
        if payload.user_ids.is_empty() { return Ok(0); }
        self.pool.execute(move |pool| async move {
            sqlx::query(
                r#"
                INSERT INTO bongas.user_interactions (user_id, item_id, interaction_type, implicit_rating, watch_duration_seconds, visitor_id, device_hash, device_type, created_at)
                SELECT * FROM unnest($1::int[], $2::int[], $3::text[], $4::float4[], $5::int[], $6::text[], $7::text[], $8::text[], $9::timestamptz[])
                "#
            )
            .bind(&payload.user_ids).bind(&payload.item_ids).bind(&payload.types).bind(&payload.ratings).bind(&payload.watch_durations)
            .bind(&payload.visitor_ids).bind(&payload.device_hashes).bind(&payload.device_types)
            .bind(&payload.timestamps)
            .execute(&pool).await.map(|r| r.rows_affected())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }

    pub async fn update_arrival_pattern(&self, user_id: i32) -> AppResult<()> {
        let hour = chrono::Utc::now().hour() as i32;
        let bit_mask = 1i64 << hour;
        self.pool.execute(|pool| async move {
            sqlx::query(
                r#"
                INSERT INTO bongas.user_arrival_patterns (user_id, hour_mask, last_active_at)
                VALUES ($1, $2, NOW())
                ON CONFLICT (user_id) DO UPDATE SET hour_mask = bongas.user_arrival_patterns.hour_mask | $2, last_active_at = NOW()
                "#
            ).bind(user_id).bind(bit_mask).execute(&pool).await.map(|_| ())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }

    pub async fn get_likely_arrivals(&self, target_hour: u32) -> AppResult<Vec<i32>> {
        let bit_mask = 1i64 << target_hour;
        self.pool.execute(|pool| async move {
            sqlx::query_as::<_, (i32,)>("SELECT user_id FROM bongas.user_arrival_patterns WHERE (hour_mask & $1) != 0")
            .bind(bit_mask).fetch_all(&pool).await.map(|rows| rows.into_iter().map(|r| r.0).collect())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query { message: e.to_string(), source: None }))
    }

    /// Get top engaged users in the last 30 days (High priority for cache warming)
    pub async fn get_top_engaged_users(&self, limit: i32) -> AppResult<Vec<i32>> {
        self.pool.execute(|pool| async move {
            sqlx::query_as::<_, (i32,)> (
                r#"
                SELECT user_id
                FROM bongas.user_interactions
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

    /// Get scenario engagement scores for a specific identity (visitor or profile).
    pub async fn get_scenario_engagement_scores(
        &self,
        profile_id: Option<&str>,
        visitor_id: Option<&str>
    ) -> AppResult<HashMap<String, f32>> {
        let vid = visitor_id.map(|s| s.to_string());
        let pid = profile_id.map(|s| s.to_string());

        self.pool.execute(move |pool| async move {
            let rows: Vec<(String, f32)> = sqlx::query_as(
                r#"
                SELECT scenario_slug, 
                       (COUNT(*) * 1.0 + SUM(CASE WHEN interaction_type = 'click' THEN 2.0 ELSE 0.0 END)) as score
                FROM bongas.user_interactions
                WHERE (profile_id = $1 AND $1 IS NOT NULL) OR (visitor_id = $2 AND $2 IS NOT NULL)
                AND created_at > NOW() - INTERVAL '7 days'
                AND scenario_slug IS NOT NULL
                GROUP BY scenario_slug
                "#
            )
            .bind(pid)
            .bind(vid)
            .fetch_all(&pool)
            .await?;

            Ok(rows.into_iter().collect())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query {
            message: format!("Failed to fetch scenario engagement scores: {}", e),
            source: None,
        }))
    }

    /// Stitch anonymous interactions (by visitor_id) to a persistent user_id and profile_id.
    pub async fn stitch_identity(&self, visitor_id: &str, user_id: i32, profile_id: &str) -> AppResult<u64> {
        let vid = visitor_id.to_string();
        let pid = profile_id.to_string();
        
        self.pool.execute(move |pool| async move {
            let res = sqlx::query(
                r#"
                UPDATE bongas.user_interactions 
                SET user_id = $1, profile_id = $2 
                WHERE visitor_id = $3 AND (user_id = 0 OR user_id IS NULL)
                "#
            )
            .bind(user_id)
            .bind(pid)
            .bind(vid)
            .execute(&pool)
            .await?;
            
            Ok(res.rows_affected())
        }).await.map_err(|e| AppError::Postgres(PostgresError::Query {
            message: format!("Failed to stitch interactions: {}", e),
            source: None,
        }))
    }
}

use std::collections::HashMap;
