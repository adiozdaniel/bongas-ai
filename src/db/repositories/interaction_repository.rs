use anyhow::Result;
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use crate::db::models::UserInteraction;

pub struct InteractionRepository {
    pool: PgPool,
}

impl InteractionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record a user interaction
    pub async fn create(
        &self,
        user_id: i32,
        item_id: i32,
        interaction_type: &str,
        watch_duration_seconds: Option<i32>,
        completion_percentage: Option<f32>,
        scenario_slug: Option<&str>,
        device_type: Option<&str>,
    ) -> Result<UserInteraction> {
        let implicit_rating = completion_percentage.map(|cp| {
            // Convert watch completion to implicit rating (0.0 - 1.0)
            (cp / 100.0).clamp(0.0, 1.0)
        });

        let interaction = sqlx::query_as::<_, UserInteraction>(
            r#"
            INSERT INTO user_interactions (
                user_id, item_id, interaction_type, watch_duration_seconds,
                completion_percentage, implicit_rating, scenario_slug, device_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(item_id)
        .bind(interaction_type)
        .bind(watch_duration_seconds)
        .bind(completion_percentage)
        .bind(implicit_rating)
        .bind(scenario_slug)
        .bind(device_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(interaction)
    }

    /// Get recent interactions for a user
    pub async fn get_user_recent(
        &self,
        user_id: i32,
        limit: i64,
    ) -> Result<Vec<UserInteraction>> {
        let interactions = sqlx::query_as::<_, UserInteraction>(
            "SELECT * FROM user_interactions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(interactions)
    }

    /// Get interactions for a user within a time window
    pub async fn get_user_interactions_since(
        &self,
        user_id: i32,
        since: DateTime<Utc>,
    ) -> Result<Vec<UserInteraction>> {
        let interactions = sqlx::query_as::<_, UserInteraction>(
            "SELECT * FROM user_interactions WHERE user_id = $1 AND created_at >= $2 ORDER BY created_at DESC"
        )
        .bind(user_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        Ok(interactions)
    }

    /// Get item IDs the user has already watched (for filtering)
    pub async fn get_watched_item_ids(&self, user_id: i32, limit: i64) -> Result<Vec<i32>> {
        let rows = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT DISTINCT item_id FROM user_interactions
            WHERE user_id = $1 AND interaction_type IN ('view', 'complete')
            ORDER BY item_id
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get interactions by type for analytics
    pub async fn get_by_type(
        &self,
        interaction_type: &str,
        since: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<UserInteraction>> {
        let interactions = sqlx::query_as::<_, UserInteraction>(
            r#"
            SELECT * FROM user_interactions
            WHERE interaction_type = $1 AND created_at >= $2
            ORDER BY created_at DESC
            LIMIT $3
            "#
        )
        .bind(interaction_type)
        .bind(since)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(interactions)
    }
}
