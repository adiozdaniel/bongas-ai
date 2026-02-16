use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::{info, warn, debug};

#[derive(Deserialize)]
struct Params {
    /// Model name to use (e.g., "two_tower", "bert4rec")
    model_name: String,
    /// Model format (for metadata purposes)
    #[serde(default = "default_model_format")]
    model_format: String,
    /// Model version (optional, defaults to latest deployed)
    #[serde(default)]
    model_version: Option<String>,
    /// Maximum number of results to return
    top_k: usize,
    /// Batch size for inference (defaults to 64)
    #[serde(default = "default_batch_size")]
    batch_size: usize,
    /// User feature dimension (must match model input)
    #[serde(default = "default_user_dim")]
    user_feature_dim: usize,
    /// Item feature dimension (must match model input)
    #[serde(default = "default_item_dim")]
    item_feature_dim: usize,
    /// Minimum score threshold (filter out low scores)
    #[serde(default)]
    min_score_threshold: Option<f32>,
}

fn default_model_format() -> String { "onnx".to_string() }
fn default_batch_size() -> usize { 64 }
fn default_user_dim() -> usize { 64 }
fn default_item_dim() -> usize { 32 }

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
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse onnx_inference params")?;

        let user_id = context.user_id
            .ok_or_else(|| anyhow::anyhow!("user_id required for ONNX inference"))?;

        if input.is_empty() {
            debug!(
                request_id = %context.request_id,
                "ONNX inference skipped: no input items"
            );
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            model_name = %params.model_name,
            model_format = %params.model_format,
            input_count = input.len(),
            user_id = user_id,
            "Starting ONNX inference stage"
        );

        // Record model inference start

        let start = std::time::Instant::now();

        // Step 1: Get ONNX model from ModelLoader
        let model = match &params.model_version {
            Some(version) => {
                context.model_loader
                    .get_model_version(&params.model_name, version)
                    .await
                    .with_context(|| format!(
                        "Failed to load model {}:{}",
                        params.model_name, version
                    ))?
            }
            None => {
                context.model_loader
                    .get_model(&params.model_name)
                    .await
                    .with_context(|| format!(
                        "Failed to load latest model {}",
                        params.model_name
                    ))?
            }
        };

        // Step 2: Get user features from FeatureStore
        let user_features = context.feature_store
            .get_user_features(user_id, params.user_feature_dim)
            .await
            .context("Failed to get user features from FeatureStore")?;

        // Step 3: Get item features for all candidates
        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let item_features_map = context.feature_store
            .get_item_features(&item_ids, params.item_feature_dim)
            .await
            .context("Failed to get item features from FeatureStore")?;

        // Step 4: Prepare batch inputs for ONNX model
        let mut user_batch: Vec<Vec<f32>> = Vec::new();
        let mut item_batch: Vec<Vec<f32>> = Vec::new();
        let mut batch_item_ids: Vec<i32> = Vec::new();

        for item in &input {
            let item_feats = item_features_map
                .get(&item.item_id)
                .cloned()
                .unwrap_or_else(|| vec![0.0; params.item_feature_dim]);

            user_batch.push(user_features.clone());
            item_batch.push(item_feats);
            batch_item_ids.push(item.item_id);
        }

        // Step 5: Run ONNX inference in batches
        let mut scores: Vec<f32> = Vec::with_capacity(batch_item_ids.len());
        
        if !user_batch.is_empty() {
            let mut engine = model.write().await;
            
            // Chunk the batch according to params.batch_size
            for chunk_idx in (0..user_batch.len()).step_by(params.batch_size) {
                let end = std::cmp::min(chunk_idx + params.batch_size, user_batch.len());
                let user_chunk = user_batch[chunk_idx..end].to_vec();
                let item_chunk = item_batch[chunk_idx..end].to_vec();
                
                let chunk_scores = engine.predict_batch(user_chunk, item_chunk)
                    .with_context(|| format!(
                        "ONNX batch inference failed for chunk {}-{}",
                        chunk_idx, end
                    ))?;
                
                scores.extend(chunk_scores);
            }
        } else {
            warn!(
                request_id = %context.request_id,
                "No valid features found for ONNX inference"
            );
            scores = vec![0.5; input.len()]; // Fallback neutral scores
        };

        let inference_time = start.elapsed();

        // Step 6: Combine scores with items
        let mut results: Vec<ScoredItem> = batch_item_ids
            .iter()
            .zip(scores.iter())
            .map(|(&item_id, &score)| {
                ScoredItem::new(
                    item_id,
                    score,
                    json!({
                        "inference_engine": "onnx",
                        "model_name": params.model_name,
                        "model_format": params.model_format,
                        "model_version": params.model_version.as_deref().unwrap_or("latest"),
                        "inference_time_ms": inference_time.as_millis() as u64
                    }),
                )
            })
            .collect();

        // Step 7: Filter by minimum score threshold
        if let Some(min_score) = params.min_score_threshold {
            let before_count = results.len();
            results.retain(|item| item.score >= min_score);
            if results.len() < before_count {
                debug!(
                    request_id = %context.request_id,
                    filtered = before_count - results.len(),
                    min_score = min_score,
                    "Filtered low-scoring items"
                );
            }
        }

        // Step 8: Sort by score descending and limit
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(params.top_k);

        info!(
            request_id = %context.request_id,
            model_name = %params.model_name,
            input_count = input.len(),
            output_count = results.len(),
            inference_ms = inference_time.as_millis() as u64,
            "ONNX inference complete"
        );

        Ok(results)
    }
}


