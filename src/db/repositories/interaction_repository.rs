use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

pub struct InteractionRepository {
    db_pool: Arc<PgPool>,
}

impl InteractionRepository {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn create_implicit_rating(
        &self,
        user_id: i32,
        item_id: i32,
        rating: f32,
        watch_duration_seconds: i32,
    ) -> Result<()> {
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
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }

    pub async fn create_explicit_rating(
        &self,
        user_id: i32,
        item_id: i32,
        rating: f32,
    ) -> Result<()> {
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
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }
}