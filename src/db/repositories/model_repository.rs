use anyhow::Result;
use sqlx::PgPool;
use crate::db::models::{ModelRegistry, OnnxSession};

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

    /// Create or update ONNX model entry
    pub async fn upsert_onnx_model(
        &self,
        model_name: &str,
        version: &str,
        onnx_model_path: &str,
        onnx_opset_version: i32,
        onnx_input_shapes: serde_json::Value,
        onnx_output_names: serde_json::Value,
    ) -> Result<ModelRegistry> {
        let model = sqlx::query_as::<_, ModelRegistry>(
            r#"
            INSERT INTO model_registry (
                model_name, version, model_format, onnx_model_path,
                onnx_opset_version, onnx_input_shapes, onnx_output_names,
                onnx_runtime_provider, status
            )
            VALUES ($1, $2, 'onnx', $3, $4, $5, $6, 'cpu', 'exported_to_onnx')
            ON CONFLICT (model_name, version) DO UPDATE SET
                onnx_model_path = EXCLUDED.onnx_model_path,
                onnx_opset_version = EXCLUDED.onnx_opset_version,
                onnx_input_shapes = EXCLUDED.onnx_input_shapes,
                onnx_output_names = EXCLUDED.onnx_output_names
            RETURNING *
            "#
        )
        .bind(model_name)
        .bind(version)
        .bind(onnx_model_path)
        .bind(onnx_opset_version)
        .bind(onnx_input_shapes)
        .bind(onnx_output_names)
        .fetch_one(&self.pool)
        .await?;

        Ok(model)
    }

    /// Deploy a model (set status to 'deployed')
    pub async fn deploy_model(&self, model_name: &str, version: &str) -> Result<ModelRegistry> {
        let model = sqlx::query_as::<_, ModelRegistry>(
            r#"
            UPDATE model_registry
            SET status = 'deployed', deployed_at = NOW()
            WHERE model_name = $1 AND version = $2
            RETURNING *
            "#
        )
        .bind(model_name)
        .bind(version)
        .fetch_one(&self.pool)
        .await?;

        Ok(model)
    }

    /// Archive a model
    pub async fn archive_model(&self, model_name: &str, version: &str) -> Result<()> {
        sqlx::query(
            "UPDATE model_registry SET status = 'archived' WHERE model_name = $1 AND version = $2"
        )
        .bind(model_name)
        .bind(version)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -- ONNX Session Management --

    /// Create or update ONNX session
    pub async fn upsert_onnx_session(
        &self,
        session_id: &str,
        model_path: &str,
        provider: &str,
        memory_pool_size_mb: i32,
        optimization_level: &str,
        load_time_ms: i32,
    ) -> Result<OnnxSession> {
        let session = sqlx::query_as::<_, OnnxSession>(
            r#"
            INSERT INTO onnx_sessions (
                session_id, model_path, provider, memory_pool_size_mb,
                optimization_level, load_time_ms, is_loaded
            )
            VALUES ($1, $2, $3, $4, $5, $6, true)
            ON CONFLICT (session_id) DO UPDATE SET
                model_path = EXCLUDED.model_path,
                provider = EXCLUDED.provider,
                memory_pool_size_mb = EXCLUDED.memory_pool_size_mb,
                optimization_level = EXCLUDED.optimization_level,
                load_time_ms = EXCLUDED.load_time_ms,
                is_loaded = true,
                last_used_at = NOW(),
                updated_at = NOW()
            RETURNING *
            "#
        )
        .bind(session_id)
        .bind(model_path)
        .bind(provider)
        .bind(memory_pool_size_mb)
        .bind(optimization_level)
        .bind(load_time_ms)
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Record ONNX session metrics
    pub async fn record_onnx_session_metrics(
        &self,
        session_id: &str,
        inference_latency_p50_ms: i32,
        inference_latency_p95_ms: i32,
        memory_usage_mb: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE onnx_sessions
            SET inference_latency_p50_ms = $2,
                inference_latency_p95_ms = $3,
                memory_usage_mb = $4,
                last_used_at = NOW(),
                updated_at = NOW()
            WHERE session_id = $1
            "#
        )
        .bind(session_id)
        .bind(inference_latency_p50_ms)
        .bind(inference_latency_p95_ms)
        .bind(memory_usage_mb)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all loaded ONNX sessions
    pub async fn get_loaded_sessions(&self) -> Result<Vec<OnnxSession>> {
        let sessions = sqlx::query_as::<_, OnnxSession>(
            "SELECT * FROM onnx_sessions WHERE is_loaded = true ORDER BY last_used_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Mark session as unloaded
    pub async fn unload_session(&self, session_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE onnx_sessions SET is_loaded = false, updated_at = NOW() WHERE session_id = $1"
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
