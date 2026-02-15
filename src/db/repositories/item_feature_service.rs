//! Unified item feature service for pipeline stages.
//!
//! Provides a single abstraction over item_features queries so that pipeline
//! stages never issue raw SQL. All queries go through the resilient pool with
//! circuit breaker + bulkhead + metrics.
//!
//! # Why this exists
//! Before this service, ~20 pipeline stages each issued their own
//! `sqlx::query_as("SELECT ... FROM item_features WHERE item_id = ANY($1)")`
//! directly on the raw PgPool, bypassing all resilience infrastructure.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::FromRow;

use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;

// ─── Row types (internal) ──────────────────────────────────────────────────

/// Full item features row — superset of all columns pipeline stages may need.
#[derive(Debug, Clone, FromRow)]
pub struct ItemFeatureRow {
    pub item_id: i32,
    // Content metadata
    pub title: Option<String>,
    pub description: Option<String>,
    pub genres: Option<JsonValue>,
    pub tags: Option<JsonValue>,
    pub creators: Option<JsonValue>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub audio_languages: Option<JsonValue>,
    pub subtitle_languages: Option<JsonValue>,
    pub age_rating: Option<String>,
    pub duration_seconds: Option<i32>,
    pub release_year: Option<i32>,
    // Scores
    pub view_count: i32,
    pub like_count: i32,
    pub completion_rate: f32,
    pub trending_score: f32,
    // Embeddings & vectors
    pub embedding: Option<Vec<f32>>,
    pub tfidf_vector: Option<JsonValue>,
}

/// User features row.
#[derive(Debug, Clone, FromRow)]
pub struct UserFeatureRow {
    pub user_id: i32,
    pub genre_affinity: Option<JsonValue>,
    pub total_watch_time_minutes: i32,
    pub total_videos_watched: i32,
    pub avg_completion_rate: f32,
    pub favorite_genres: Option<JsonValue>,
    pub embedding: Option<Vec<f32>>,
}

/// User interaction row for "already watched" queries.
#[derive(Debug, Clone, FromRow)]
pub struct WatchedItemRow {
    pub item_id: i32,
}

// ─── Service ───────────────────────────────────────────────────────────────

/// Resilient item feature service used by all pipeline stages.
///
/// Every query goes through `ResilientPool` → circuit breaker + bulkhead + timeout.
pub struct ItemFeatureService {
    pool: Arc<ResilientPool>,
    metrics: Arc<ResilienceMetricsCollector>,
}

impl ItemFeatureService {
    pub fn new(pool: Arc<ResilientPool>, metrics: Arc<ResilienceMetricsCollector>) -> Self {
        Self { pool, metrics }
    }

    // ── Item feature queries ───────────────────────────────────────────

    /// Batch-fetch full item features for a set of item IDs.
    ///
    /// This is the primary method used by filter, boost, diversify, and enrich
    /// stages. Returns a HashMap keyed by item_id for O(1) lookup.
    pub async fn get_item_features_batch(
        &self,
        item_ids: &[i32],
    ) -> Result<HashMap<i32, ItemFeatureRow>> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids = item_ids.to_vec();
        let start = std::time::Instant::now();

        let rows: Vec<ItemFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, ItemFeatureRow>(
                    r#"
                    SELECT item_id, title, description, genres, tags, creators,
                           content_type, language, audio_languages, subtitle_languages,
                           age_rating, duration_seconds, release_year,
                           view_count, like_count, completion_rate, trending_score,
                           embedding, tfidf_vector
                    FROM item_features
                    WHERE item_id = ANY($1)
                    "#,
                )
                .bind(&ids)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| (r.item_id, r)).collect())
    }

    // ── User feature queries ───────────────────────────────────────────

    /// Get user features for a single user.
    pub async fn get_user_features(&self, user_id: i32) -> Result<Option<UserFeatureRow>> {
        let start = std::time::Instant::now();

        let row: Option<UserFeatureRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, UserFeatureRow>(
                    r#"
                    SELECT user_id, genre_affinity, total_watch_time_minutes,
                           total_videos_watched, avg_completion_rate,
                           favorite_genres, embedding
                    FROM user_features
                    WHERE user_id = $1
                    "#,
                )
                .bind(user_id)
                .fetch_optional(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.user");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(row)
    }

    // ── Interaction queries ────────────────────────────────────────────

    /// Get set of item IDs the user has fully watched (completion > 90%).
    pub async fn get_watched_item_ids(&self, user_id: i32) -> Result<Vec<i32>> {
        let start = std::time::Instant::now();

        let rows: Vec<WatchedItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, WatchedItemRow>(
                    r#"
                    SELECT DISTINCT item_id
                    FROM user_interactions
                    WHERE user_id = $1
                        AND interaction_type = 'view'
                        AND completion_percentage > 0.9
                    "#,
                )
                .bind(user_id)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.watched");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }

    /// Get recent watches for a user within a time window.
    pub async fn get_recent_watches(
        &self,
        user_id: i32,
        hours: i32,
        limit: i64,
    ) -> Result<Vec<RecentWatchRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<RecentWatchRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, RecentWatchRow>(
                    r#"
                    SELECT item_id, MAX(created_at) as last_watched
                    FROM user_interactions
                    WHERE user_id = $1
                        AND interaction_type = 'view'
                        AND created_at >= NOW() - make_interval(hours => $2)
                    GROUP BY item_id
                    ORDER BY last_watched DESC
                    LIMIT $3
                    "#,
                )
                .bind(user_id)
                .bind(hours)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.recent_watches");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    // ── Fetch queries (for fetch stages) ───────────────────────────────

    /// Fetch popular content ordered by view count.
    pub async fn get_popular_content(
        &self,
        min_views: i32,
        limit: i64,
    ) -> Result<Vec<PopularItemRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<PopularItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, PopularItemRow>(
                    r#"
                    SELECT item_id, view_count, trending_score, completion_rate
                    FROM item_features
                    WHERE view_count >= $1
                    ORDER BY view_count DESC
                    LIMIT $2
                    "#,
                )
                .bind(min_views)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.popular");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch new releases.
    pub async fn get_new_releases(&self, days: i32, limit: i64) -> Result<Vec<NewReleaseRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<NewReleaseRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, NewReleaseRow>(
                    r#"
                    SELECT item_id, title, published_at, trending_score
                    FROM item_features
                    WHERE published_at >= NOW() - make_interval(days => $1)
                    ORDER BY published_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(days)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.new_releases");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch items by genre using JSONB containment.
    pub async fn get_items_by_genre(
        &self,
        genre_json: JsonValue,
        limit: i64,
    ) -> Result<Vec<GenreItemRow>> {
        let start = std::time::Instant::now();

        let rows: Vec<GenreItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, GenreItemRow>(
                    r#"
                    SELECT item_id, title, trending_score as popularity_score
                    FROM item_features
                    WHERE genres @> $1::jsonb
                    ORDER BY trending_score DESC NULLS LAST
                    LIMIT $2
                    "#,
                )
                .bind(&genre_json)
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.by_genre");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }

    /// Fetch seasonal content by tags.
    pub async fn get_seasonal_content(
        &self,
        season: &str,
        limit: i64,
    ) -> Result<Vec<SeasonalItemRow>> {
        let season = season.to_string();
        let start = std::time::Instant::now();

        let rows: Vec<SeasonalItemRow> = self
            .pool
            .execute(|pool| async move {
                sqlx::query_as::<_, SeasonalItemRow>(
                    r#"
                    SELECT item_id, title, seasonal_tags, trending_score
                    FROM item_features
                    WHERE seasonal_tags @> $1::jsonb
                    ORDER BY trending_score DESC NULLS LAST
                    LIMIT $2
                    "#,
                )
                .bind(serde_json::json!([season]))
                .bind(limit)
                .fetch_all(&pool)
                .await
            })
            .await?;

        let duration = start.elapsed();
        let metrics = self.metrics.registry().get_or_create("item_feature_service.seasonal");
        metrics.latency.record_duration(duration);
        metrics.successes.increment();

        Ok(rows)
    }
}

// ─── Additional row types ──────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct RecentWatchRow {
    pub item_id: i32,
    pub last_watched: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PopularItemRow {
    pub item_id: i32,
    pub view_count: i32,
    pub trending_score: f32,
    pub completion_rate: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct NewReleaseRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub trending_score: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct GenreItemRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub popularity_score: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SeasonalItemRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub seasonal_tags: Option<JsonValue>,
    pub trending_score: f32,
}
