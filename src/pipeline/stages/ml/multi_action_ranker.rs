use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use ndarray::Array2;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::info;

#[derive(Deserialize)]
struct Params {
    /// Model name to use (must be a multi-head model)
    model_name: String,
    /// Weights for each engagement head
    /// e.g. {"like": 5.0, "reply": 10.0, "retweet": 8.0, "report": -50.0}
    engagement_weights: HashMap<String, f32>,
    /// Mapping of output index to engagement type
    /// e.g. {0: "like", 1: "reply", ...}
    head_mapping: HashMap<usize, String>,
    /// Maximum number of results to return
    top_k: usize,
    /// Batch size for inference
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    /// User feature dimension
    #[serde(default = "default_user_dim")]
    user_feature_dim: usize,
    /// Item feature dimension
    #[serde(default = "default_item_dim")]
    item_feature_dim: usize,
}

fn default_batch_size() -> usize { 64 }
fn default_user_dim() -> usize { 64 }
fn default_item_dim() -> usize { 32 }

pub struct MultiActionRankerStage;

#[async_trait]
impl PipelineStage for MultiActionRankerStage {
    fn name(&self) -> &str {
        "multi_action_ranker"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse multi_action_ranker params")?;

        let user_id = context.user_id
            .ok_or_else(|| anyhow::anyhow!("user_id required for multi-action ranking"))?;

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let start = std::time::Instant::now();

        // 1. Get Model
        let model = context.model_loader
            .get_model(&params.model_name)
            .await
            .with_context(|| format!("Failed to load multi-action model {}", params.model_name))?;

        // 2. Get User Features
        let user_features = context.feature_store
            .get_user_features(user_id, params.user_feature_dim)
            .await?;

        // 3. Get Item Features
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features_map = context.feature_store
            .get_item_features(&item_ids, params.item_feature_dim)
            .await?;

        // 4. Run Multi-Action Inference in Batches
        let mut all_action_probs = Vec::with_capacity(input.len());

        for chunk in input.chunks(params.batch_size) {
            let mut user_batch = Array2::<f32>::zeros((chunk.len(), params.user_feature_dim));
            let mut item_batch = Array2::<f32>::zeros((chunk.len(), params.item_feature_dim));

            for (i, item) in chunk.iter().enumerate() {
                let item_feats = item_features_map
                    .get(&item.item_id)
                    .cloned()
                    .unwrap_or_else(|| vec![0.0; params.item_feature_dim]);

                for j in 0..params.user_feature_dim {
                    user_batch[[i, j]] = user_features[j];
                }
                for j in 0..params.item_feature_dim {
                    item_batch[[i, j]] = item_feats[j];
                }
            }

            let chunk_probs = model.clone().predict_multi_action(user_batch, item_batch).await
                .context("Multi-action batch inference failed")?;
            
            all_action_probs.extend(chunk_probs);
        }

        // 5. Compute Weighted Scores (Grok-style) + Skip Burner (Phase 11)
        let mut scored_results: Vec<ScoredItem> = Vec::with_capacity(input.len());
        let genre_penalties = context.cache_manager.get::<HashMap<String, f32>>(&format!("penalties:{}", user_id)).await?.unwrap_or_default();
        
        for (i, item) in input.iter().enumerate() {
            let action_probs: &Vec<f32> = &all_action_probs[i];
            let mut final_score: f32 = 0.0;
            let mut debug_info = HashMap::new();

            for (head_idx, prob) in action_probs.iter().enumerate() {
                if let Some(engagement_type) = params.head_mapping.get(&head_idx) {
                    let weight = params.engagement_weights.get(engagement_type).cloned().unwrap_or(0.0);
                    final_score += prob * weight;
                    debug_info.insert(engagement_type.clone(), *prob);
                }
            }

            // Apply Skip Burner Penalty
            if let Some(genres) = item.metadata.get("genres").and_then(|v| v.as_array()) {
                for genre in genres {
                    if let Some(g_str) = genre.as_str() {
                        if let Some(penalty) = genre_penalties.get(g_str) {
                            final_score *= penalty;
                            debug_info.insert(format!("penalty_{}", g_str), *penalty);
                        }
                    }
                }
            }

            scored_results.push(ScoredItem::new(
                item.item_id,
                final_score,
                json!({
                    "model": params.model_name,
                    "engagements": debug_info,
                    "inference_ms": start.elapsed().as_millis() as u64
                }),
            ));
        }

        // 6. Sort and Truncate
        scored_results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored_results.truncate(params.top_k);

        info!(
            request_id = %context.request_id,
            model = %params.model_name,
            output_count = scored_results.len(),
            latency_ms = start.elapsed().as_millis() as u64,
            "Multi-action ranking complete"
        );

        Ok(scored_results)
    }
}
