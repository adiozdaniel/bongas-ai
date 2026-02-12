use anyhow::Result;
use sqlx::PgPool;
use crate::db::models::ModelRegistry;

pub struct ModelRepository {
    pool: PgPool,
}

impl ModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get deployed ONNX models
    pub async fn get_deployed_onnx_models(&self) -> Result<Vec<ModelRegistry>> {
        let models = sqlx::query_as::<_, ModelRegistry>(
            r#"
            SELECT * FROM model_registry
            WHERE status = 'deployed'
            AND model_format = 'onnx'
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(models)
    }

    /// Get ONNX model by name and version
    pub async fn get_onnx_model(
        &self,
        model_name: &str,
        version: &str,
    ) -> Result<Option<ModelRegistry>> {
        let model = sqlx::query_as::<_, ModelRegistry>(
            r#"
            SELECT * FROM model_registry
            WHERE model_name = $1
            AND version = $2
            AND model_format = 'onnx'
            "#
        )
        .bind(model_name)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        Ok(model)
    }

    /// Get latest deployed model by name
    pub async fn get_latest_deployed(&self, model_name: &str) -> Result<Option<ModelRegistry>> {
        let model = sqlx::query_as::<_, ModelRegistry>(
            r#"
            SELECT * FROM model_registry
            WHERE model_name = $1
            AND status = 'deployed'
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(model_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(model)
    }
}
