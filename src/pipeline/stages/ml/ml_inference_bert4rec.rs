use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    sequence_length: usize,
    top_k: usize,
    
    use_onnx: Option<bool>,
}

pub struct MLInferenceBERT4RecStage;

#[async_trait]
impl PipelineStage for MLInferenceBERT4RecStage {
    fn name(&self) -> &str {
        "ml_inference_bert4rec"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        _input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        info!(
            request_id = %context.request_id,
            user_id = user_id,
            sequence_length = params.sequence_length,
            "Running BERT4Rec inference"
        );

        // Get user's recent interaction sequence
        let sequence = self.get_user_sequence(context, user_id, params.sequence_length).await?;

        if sequence.is_empty() {
            return Ok(vec![]);
        }

        // In production, this would run ONNX inference with the BERT4Rec model
        // For now, fetch items similar to the most recent items in the sequence
        let recent_items = &sequence[..std::cmp::min(5, sequence.len())];

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            trending_score: f32,
        }

        let candidates: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, trending_score
            FROM item_features
            WHERE item_id != ALL($1)
            ORDER BY trending_score DESC
            LIMIT $2
            "#,
        )
        .bind(recent_items)
        .bind(params.top_k as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let items: Vec<ScoredItem> = candidates.into_iter().map(|row| {
            ScoredItem {
                item_id: row.item_id,
                score: row.trending_score,
                metadata: json!({
                    "model": "bert4rec",
                    "inference_engine": "onnx",
                    "sequence_length": sequence.len(),
                }),
            }
        }).collect();

        Ok(items)
    }
}

impl MLInferenceBERT4RecStage {
    async fn get_user_sequence(
        &self,
        context: &ExecutionContext,
        user_id: i32,
        limit: usize,
    ) -> Result<Vec<i32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id
            FROM user_interactions
            WHERE user_id = $1 AND interaction_type = 'view'
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        Ok(rows.into_iter().map(|r| r.item_id).collect())
    }
}
