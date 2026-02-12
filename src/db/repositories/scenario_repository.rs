use anyhow::Result;
use sqlx::PgPool;
use crate::db::models::ScenarioConfig;

pub struct ScenarioRepository {
    pool: PgPool,
}

impl ScenarioRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load all enabled scenarios from database
    pub async fn find_all_enabled(&self) -> Result<Vec<ScenarioConfig>> {
        let scenarios = sqlx::query_as::<_, ScenarioConfig>(
            "SELECT * FROM scenario_configs WHERE enabled = true ORDER BY priority DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(scenarios)
    }
}
