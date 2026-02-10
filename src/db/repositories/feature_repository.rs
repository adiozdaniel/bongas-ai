use anyhow::Result;
use sqlx::{PgPool, Row};
use crate::db::models::{UserFeatures, ItemFeatures};

pub struct FeatureRepository {
    pool: PgPool,
}

impl FeatureRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get user features
    pub async fn get_user_features(&self, user_id: i32) -> Result<Option<UserFeatures>> {
        let features = sqlx::query_as::<_, UserFeatures>(
            "SELECT * FROM user_features WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(features)
    }

    /// Batch get user features
    pub async fn get_user_features_batch(&self, user_ids: &[i32]) -> Result<Vec<UserFeatures>> {
        let features = sqlx::query_as::<_, UserFeatures>(
            "SELECT * FROM user_features WHERE user_id = ANY($1)"
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(features)
    }

    /// Get item features by ID
    pub async fn get_item_features(&self, item_id: i32) -> Result<Option<ItemFeatures>> {
        let features = sqlx::query_as::<_, ItemFeatures>(
            "SELECT * FROM item_features WHERE item_id = $1"
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(features)
    }

    /// Batch get item features
    pub async fn get_item_features_batch(&self, item_ids: &[i32]) -> Result<Vec<ItemFeatures>> {
        let features = sqlx::query_as::<_, ItemFeatures>(
            "SELECT * FROM item_features WHERE item_id = ANY($1)"
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(features)
    }

    /// Get item features optimized for ONNX inference (tensor format)
    pub async fn get_item_features_for_onnx(&self, item_ids: &[i32]) -> Result<Vec<Vec<f32>>> {
        let rows = sqlx::query(
            r#"
            SELECT tfidf_vector FROM item_features
            WHERE item_id = ANY($1)
            ORDER BY item_id
            "#
        )
        .bind(item_ids)
        .fetch_all(&self.pool)
        .await?;

        // Convert JSONB vectors to f32 arrays for ONNX tensor input
        let mut tensors = Vec::with_capacity(rows.len());
        for row in &rows {
            let tfidf: Option<serde_json::Value> = row.get("tfidf_vector");
            if let Some(serde_json::Value::Array(arr)) = tfidf {
                let vec: Vec<f32> = arr.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                tensors.push(vec);
            } else {
                tensors.push(vec![0.0; 128]); // Default embedding dimension
            }
        }

        Ok(tensors)
    }

    /// Upsert user features (called by background worker)
    pub async fn upsert_user_features(
        &self,
        user_id: i32,
        features: &UserFeatures,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO user_features (
                user_id, genre_affinity, total_watch_time_minutes,
                total_videos_watched, avg_completion_rate, favorite_genres,
                watch_patterns, features_updated_at, last_interaction_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8)
            ON CONFLICT (user_id) DO UPDATE SET
                genre_affinity = EXCLUDED.genre_affinity,
                total_watch_time_minutes = EXCLUDED.total_watch_time_minutes,
                total_videos_watched = EXCLUDED.total_videos_watched,
                avg_completion_rate = EXCLUDED.avg_completion_rate,
                favorite_genres = EXCLUDED.favorite_genres,
                watch_patterns = EXCLUDED.watch_patterns,
                features_updated_at = NOW(),
                last_interaction_at = EXCLUDED.last_interaction_at
            "#
        )
        .bind(user_id)
        .bind(&features.genre_affinity)
        .bind(features.total_watch_time_minutes)
        .bind(features.total_videos_watched)
        .bind(features.avg_completion_rate)
        .bind(&features.favorite_genres)
        .bind(&features.watch_patterns)
        .bind(features.last_interaction_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Upsert item features (called by background worker)
    pub async fn upsert_item_features(
        &self,
        item_id: i32,
        features: &ItemFeatures,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO item_features (
                item_id, title, description, genres, tags, duration_seconds,
                tfidf_vector, view_count, like_count, completion_rate,
                trending_score, published_at, features_updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
            ON CONFLICT (item_id) DO UPDATE SET
                title = EXCLUDED.title,
                description = EXCLUDED.description,
                genres = EXCLUDED.genres,
                tags = EXCLUDED.tags,
                duration_seconds = EXCLUDED.duration_seconds,
                tfidf_vector = EXCLUDED.tfidf_vector,
                view_count = EXCLUDED.view_count,
                like_count = EXCLUDED.like_count,
                completion_rate = EXCLUDED.completion_rate,
                trending_score = EXCLUDED.trending_score,
                published_at = EXCLUDED.published_at,
                features_updated_at = NOW()
            "#
        )
        .bind(item_id)
        .bind(&features.title)
        .bind(&features.description)
        .bind(&features.genres)
        .bind(&features.tags)
        .bind(features.duration_seconds)
        .bind(&features.tfidf_vector)
        .bind(features.view_count)
        .bind(features.like_count)
        .bind(features.completion_rate)
        .bind(features.trending_score)
        .bind(features.published_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get trending items
    pub async fn get_trending_items(&self, limit: i64) -> Result<Vec<ItemFeatures>> {
        let items = sqlx::query_as::<_, ItemFeatures>(
            "SELECT * FROM item_features ORDER BY trending_score DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(items)
    }
}
