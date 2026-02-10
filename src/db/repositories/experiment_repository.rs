use anyhow::Result;
use sqlx::PgPool;
use serde_json::Value as JsonValue;
use crate::db::models::Experiment;

pub struct ExperimentRepository {
    pool: PgPool,
}

impl ExperimentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all running experiments
    pub async fn find_running(&self) -> Result<Vec<Experiment>> {
        let experiments = sqlx::query_as::<_, Experiment>(
            "SELECT * FROM experiments WHERE status = 'running' ORDER BY started_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(experiments)
    }

    /// Find experiment by ID
    pub async fn find_by_id(&self, id: i32) -> Result<Option<Experiment>> {
        let experiment = sqlx::query_as::<_, Experiment>(
            "SELECT * FROM experiments WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(experiment)
    }

    /// Create a new experiment
    pub async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        hypothesis: Option<&str>,
        variants: JsonValue,
        assignment_method: &str,
        created_by: &str,
    ) -> Result<Experiment> {
        let experiment = sqlx::query_as::<_, Experiment>(
            r#"
            INSERT INTO experiments (name, description, hypothesis, variants, assignment_method, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(name)
        .bind(description)
        .bind(hypothesis)
        .bind(variants)
        .bind(assignment_method)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(experiment)
    }

    /// Start an experiment
    pub async fn start(&self, id: i32) -> Result<Experiment> {
        let experiment = sqlx::query_as::<_, Experiment>(
            r#"
            UPDATE experiments
            SET status = 'running', started_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(experiment)
    }

    /// Complete an experiment with results
    pub async fn complete(
        &self,
        id: i32,
        results: JsonValue,
        winner_variant_id: Option<&str>,
    ) -> Result<Experiment> {
        let experiment = sqlx::query_as::<_, Experiment>(
            r#"
            UPDATE experiments
            SET status = 'completed', ended_at = NOW(), results = $2, winner_variant_id = $3
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(results)
        .bind(winner_variant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(experiment)
    }

    /// Archive an experiment
    pub async fn archive(&self, id: i32) -> Result<()> {
        sqlx::query("UPDATE experiments SET status = 'archived' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
