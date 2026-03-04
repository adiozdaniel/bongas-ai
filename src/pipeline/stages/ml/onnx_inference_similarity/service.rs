use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use std::collections::HashMap;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::{info, debug};

#[derive(Deserialize)]
struct Params {
    /// Model name for embedding extraction (optional - uses precomputed if not specified)
    #[serde(default)]
    model_name: Option<String>,
    /// Similarity method: "cosine", "dot_product", or "euclidean"
    #[serde(default = "default_method")]
    method: String,
    /// Number of similar items to return per seed item
    #[serde(default = "default_top_k_per_seed")]
    top_k_per_seed: usize,
    /// Total maximum results to return
    #[serde(default = "default_max_results")]
    max_results: usize,
    /// Embedding dimension
    #[serde(default = "default_embedding_dim")]
    embedding_dim: usize,
    /// Minimum similarity threshold
    #[serde(default)]
    min_similarity: Option<f32>,
    /// Number of candidate items to consider
    #[serde(default = "default_candidate_limit")]
    candidate_limit: usize,
}

fn default_method() -> String { "cosine".to_string() }
fn default_top_k_per_seed() -> usize { 10 }
fn default_max_results() -> usize { 50 }
fn default_embedding_dim() -> usize { 128 }
fn default_candidate_limit() -> usize { 1000 }

pub struct ONNXInferenceSimilarityStage;

#[async_trait]
impl PipelineStage for ONNXInferenceSimilarityStage {
    fn name(&self) -> &str {
        "onnx_inference_similarity"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse onnx_inference_similarity params")?;

        if input.is_empty() {
            debug!(
                request_id = %context.request_id,
                "ONNX similarity skipped: no seed items"
            );
            return Ok(Vec::new());
        }

        info!(
            request_id = %context.request_id,
            seed_count = input.len(),
            method = %params.method,
            model = ?params.model_name,
            "Running ONNX similarity inference"
        );

        let start = std::time::Instant::now();

        // Get seed item embeddings
        let seed_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let seed_embeddings = context.embedding_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EmbeddingManager not configured"))?
            .get_item_embeddings(&seed_ids, params.embedding_dim)
            .await?;

        // Get candidate items (excluding seeds)
        let candidate_ids = context.item_feature_service
            .get_candidate_item_ids_for_similarity(&seed_ids, params.candidate_limit as i64)
            .await?;
        let candidate_embeddings = context.embedding_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EmbeddingManager not configured"))?
            .get_item_embeddings(&candidate_ids, params.embedding_dim)
            .await?;

        // Compute similarities
        let mut results: Vec<ScoredItem> = Vec::new();

        // Convert seed embeddings to a map for lookup
        let seed_emb_map: HashMap<i32, Vec<f32>> = seed_embeddings.into_iter().collect();
        let cand_emb_map: HashMap<i32, Vec<f32>> = candidate_embeddings.into_iter().collect();

        for &seed_id in &seed_ids {
            let seed_vec = match seed_emb_map.get(&seed_id) {
                Some(v) => v,
                None => continue,
            };

            let mut seed_results: Vec<ScoredItem> = cand_emb_map
                .iter()
                .filter(|(cid, _)| !seed_ids.contains(cid))
                .map(|(cid, cand_vec)| {
                    let score = match params.method.as_str() {
                        "cosine" => crate::ml::assets::utils::service::cosine_similarity(seed_vec, cand_vec),
                        "dot_product" => crate::ml::assets::utils::service::dot_product(seed_vec, cand_vec),
                        "euclidean" => crate::ml::assets::utils::service::euclidean_similarity(seed_vec, cand_vec),
                        _ => crate::ml::assets::utils::service::cosine_similarity(seed_vec, cand_vec),
                    };
                    ScoredItem::new(
                        *cid,
                        score,
                        json!({
                            "seed_item_id": seed_id,
                            "similarity_method": params.method,
                            "inference_engine": "onnx_similarity"
                        }),
                    )
                })
                .collect();

            // Filter by minimum similarity
            if let Some(min_sim) = params.min_similarity {
                seed_results.retain(|item| item.score >= min_sim);
            }

            // Sort and limit per seed
            seed_results.sort_by(|a, b| {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            });
            seed_results.truncate(params.top_k_per_seed);
            results.extend(seed_results);
        }

        // Deduplicate by item_id (keep highest score)
        let mut seen: HashMap<i32, usize> = HashMap::new();
        let mut deduped: Vec<ScoredItem> = Vec::new();

        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        for item in results {
            if !seen.contains_key(&item.item_id) {
                seen.insert(item.item_id, deduped.len());
                deduped.push(item);
            }
        }

        // Limit total results
        deduped.truncate(params.max_results);

        let inference_time = start.elapsed();

        info!(
            request_id = %context.request_id,
            seed_count = seed_ids.len(),
            candidate_count = candidate_ids.len(),
            output_count = deduped.len(),
            inference_ms = inference_time.as_millis() as u64,
            "ONNX similarity inference complete"
        );

        Ok(deduped)
    }
}
