use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    model_name: String,
    model_format: String,
    #[allow(dead_code)]
    model_path: Option<String>,
    top_k: usize,
    #[allow(dead_code)]
    batch_size: Option<usize>,
}

pub struct ONNXInferenceStage;

#[async_trait]
impl PipelineStage for ONNXInferenceStage {
    fn name(&self) -> &str {
        "onnx_inference"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;

        info!(
            request_id = %context.request_id,
            model_name = %params.model_name,
            model_format = %params.model_format,
            input_count = input.len(),
            user_id = user_id,
            "Starting ONNX inference stage"
        );

        // Get user features from database
        let user_features = self.get_user_features(context, user_id).await?;

        // Get item features for candidates
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features_map = self.get_item_features(context, &item_ids).await?;

        // Score each candidate using feature dot product (placeholder for ONNX inference)
        // In production, this calls ort::Session for actual ONNX model inference
        let mut results: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            let item_feats = item_features_map
                .iter()
                .find(|(id, _)| *id == item.item_id)
                .map(|(_, feats)| feats.as_slice());

            let score = if let Some(item_feats) = item_feats {
                // Simplified dot product scoring
                user_features.iter()
                    .zip(item_feats.iter())
                    .map(|(u, i)| u * i)
                    .sum::<f32>()
            } else {
                0.0
            };

            item.score = score;
            item.metadata["inference_engine"] = json!("onnx");
            item.metadata["model_name"] = json!(params.model_name);
            item
        }).collect();

        // Sort by score descending and take top_k
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(params.top_k);

        info!(
            request_id = %context.request_id,
            output_count = results.len(),
            "ONNX inference complete"
        );

        Ok(results)
    }
}

impl ONNXInferenceStage {
    async fn get_user_features(
        &self,
        context: &ExecutionContext,
        user_id: i32,
    ) -> Result<Vec<f32>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            genre_affinity: Option<JsonValue>,
            total_watch_time_minutes: i32,
            avg_completion_rate: f32,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT genre_affinity, total_watch_time_minutes, avg_completion_rate
            FROM user_features
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(context.db_pool.as_ref())
        .await?;

        match row {
            Some(row) => {
                let mut features = Vec::new();

                if let Some(genre_affinity) = row.genre_affinity {
                    if let Ok(affinity) = serde_json::from_value::<Vec<f32>>(genre_affinity) {
                        features.extend(affinity);
                    }
                }

                features.push(row.total_watch_time_minutes as f32);
                features.push(row.avg_completion_rate);

                Ok(features)
            }
            None => {
                // Default features for new users
                Ok(vec![0.0; 64])
            }
        }
    }

    async fn get_item_features(
        &self,
        context: &ExecutionContext,
        item_ids: &[i32],
    ) -> Result<Vec<(i32, Vec<f32>)>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: i32,
            tfidf_vector: Option<JsonValue>,
            view_count: i32,
            trending_score: f32,
            completion_rate: f32,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT item_id, tfidf_vector, view_count, trending_score, completion_rate
            FROM item_features
            WHERE item_id = ANY($1)
            ORDER BY item_id
            "#,
        )
        .bind(item_ids)
        .fetch_all(context.db_pool.as_ref())
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let mut features = Vec::new();

            if let Some(tfidf) = row.tfidf_vector {
                if let Ok(vec) = serde_json::from_value::<Vec<f32>>(tfidf) {
                    features.extend(vec);
                }
            }

            features.push(row.view_count as f32);
            features.push(row.trending_score);
            features.push(row.completion_rate);

            result.push((row.item_id, features));
        }

        Ok(result)
    }
}
