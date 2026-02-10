use anyhow::Result;
use sqlx::PgPool;
use crate::db::models::{ScenarioConfig, PipelineDefinition};

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

    /// Find scenario by slug
    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<ScenarioConfig>> {
        let scenario = sqlx::query_as::<_, ScenarioConfig>(
            "SELECT * FROM scenario_configs WHERE slug = $1 AND enabled = true"
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(scenario)
    }

    /// Find scenarios using ONNX inference
    pub async fn find_with_onnx_inference(&self) -> Result<Vec<ScenarioConfig>> {
        let scenarios = sqlx::query_as::<_, ScenarioConfig>(
            r#"
            SELECT * FROM scenario_configs
            WHERE enabled = true
            AND pipeline @> '{"stages": [{"type": "onnx_inference"}]}'::jsonb
            ORDER BY priority DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(scenarios)
    }

    /// Create new scenario
    pub async fn create(
        &self,
        slug: &str,
        name: &str,
        pipeline: &PipelineDefinition,
        created_by: &str,
    ) -> Result<ScenarioConfig> {
        let pipeline_json = serde_json::to_value(pipeline)?;

        let scenario = sqlx::query_as::<_, ScenarioConfig>(
            r#"
            INSERT INTO scenario_configs (slug, name, pipeline, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(slug)
        .bind(name)
        .bind(pipeline_json)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(scenario)
    }

    /// Update scenario pipeline
    pub async fn update_pipeline(
        &self,
        slug: &str,
        pipeline: &PipelineDefinition,
    ) -> Result<ScenarioConfig> {
        let pipeline_json = serde_json::to_value(pipeline)?;

        let scenario = sqlx::query_as::<_, ScenarioConfig>(
            r#"
            UPDATE scenario_configs
            SET pipeline = $2, version = version + 1, updated_at = NOW()
            WHERE slug = $1
            RETURNING *
            "#
        )
        .bind(slug)
        .bind(pipeline_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(scenario)
    }

    /// Delete scenario
    pub async fn delete(&self, slug: &str) -> Result<()> {
        sqlx::query("DELETE FROM scenario_configs WHERE slug = $1")
            .bind(slug)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
