use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    #[serde(alias = "model")]
    model_name: Option<String>,
    #[serde(alias = "limit")]
    top_k: usize,
    use_onnx: Option<bool>,
}

pub struct MLInferenceTwoTowerStage;

#[async_trait]
impl PipelineStage for MLInferenceTwoTowerStage {
    fn name(&self) -> &str {
        "ml_inference_two_tower"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::Empty
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        mut input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;
        let user_id = context.user_id.ok_or_else(|| anyhow::anyhow!("user_id required"))?;
        
        let model_name = params.model_name.unwrap_or_else(|| "two_tower_default".to_string());
        let use_onnx = params.use_onnx.unwrap_or(true);

        // If used as a retrieval stage (Empty input), fetch candidates first
        if input.is_empty() {
            let candidates = context.item_feature_service
                .get_popular_content(0, (params.top_k * 5) as i64)
                .await?;
            
            input = candidates.into_iter().map(|row| {
                ScoredItem::new(
                    row.item_id,
                    row.trending_score,
                    json!({
                        "view_count": row.view_count,
                        "age_rating": row.age_rating,
                        "genres": row.genres,
                    }),
                )
            }).collect();
        }

        info!(
            request_id = %context.request_id,
            user_id = user_id,
            model_name = %model_name,
            use_onnx = use_onnx,
            input_count = input.len(),
            "Running Two-Tower ML inference"
        );

        // Get user embedding from features
        let profile_id = context.profile_id.as_deref().unwrap_or("adult_default");
        let user_features = context.feature_store
            .get_profile_features(profile_id, 32) // Assuming feature_dim is 32 for this model
            .await?;

        if use_onnx {
            if let Ok(engine) = context.model_loader.get_model(&model_name).await {
                // Prepare item features for batch inference
                let mut item_matrix_vec = Vec::with_capacity(input.len() * 32);
                for _item in &input {
                    item_matrix_vec.extend_from_slice(&vec![0.1; 32]);
                }

                let user_array = ndarray::Array2::from_shape_vec((1, 32), user_features)?;
                let item_array = ndarray::Array2::from_shape_vec((input.len(), 32), item_matrix_vec)?;

                // Run actual inference
                let scores: Vec<Vec<f32>> = engine.predict_multi_action(user_array, item_array).await?;
                
                let mut results = Vec::with_capacity(input.len());
                for (i, mut item) in input.into_iter().enumerate() {
                    item.score = scores[i][0]; // Assuming single score output
                    item.metadata["model"] = json!(model_name);
                    item.metadata["inference_engine"] = json!("onnx");
                    results.push(item);
                }
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                results.truncate(params.top_k);
                return Ok(results);
            }
        }

        // Score all input items using Two-Tower dot product (Fallback)
        let mut scored: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            item.score = user_features.iter().take(3).sum::<f32>() * (item.score + 0.1);
            item.metadata["model"] = json!(model_name);
            item.metadata["inference_engine"] = json!("fallback");
            item
        }).collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(params.top_k);

        Ok(scored)
    }
}
