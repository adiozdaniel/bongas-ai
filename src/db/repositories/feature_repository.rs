use anyhow::Result;
use sqlx::PgPool;
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
