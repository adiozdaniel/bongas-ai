pub mod models;
pub mod repositories;

use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};
use crate::config::settings::DatabaseSettings;

/// Create PostgreSQL connection pool
pub async fn create_pool(config: &DatabaseSettings) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&config.url)
        .await?;

    Ok(pool)
}
