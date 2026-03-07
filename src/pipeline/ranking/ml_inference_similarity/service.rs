use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::service::ExecutionContext;
use crate::db::ItemFeatureRow;
use crate::ml::assets::utils::service::{cosine_similarity, dot_product};
use tracing::info;

#[derive(Deserialize)]
struct Params {
    method: String,
    top_k: usize,
}

pub struct MLInferenceSimilarityStage;

#[async_trait]
impl PipelineStage for MLInferenceSimilarityStage {
    fn name(&self) -> &str {
        "ml_inference_similarity"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())?;

        info!(
            request_id = %context.request_id,
            method = %params.method,
            input_count = input.len(),
            "Running native similarity inference"
        );

        if input.is_empty() {
            return Ok(vec![]);
        }

        // Get embeddings for seed items
        let seed_ids: Vec<i32> = input.iter().map(|i| i.item_id).collect();
        let seed_embeddings_tuples = context.embedding_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EmbeddingManager not configured"))?
            .get_item_embeddings(&seed_ids, 128) // 128 is the feature dimension from `unwrap_or_else(|| vec![0.0; 128])`
            .await?;
        let seed_embeddings: Vec<Vec<f32>> = seed_embeddings_tuples.into_iter().map(|(_, emb)| emb).collect();

        // Get candidate item features
        let all_item_ids_in_input: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        let all_item_features_map = context.item_feature_service
            .get_item_features_batch(&all_item_ids_in_input)
            .await?;

        // Filter out seed items and items without dense embedding
        let candidates_features: Vec<ItemFeatureRow> = all_item_features_map.into_values()
            .filter(|feature_row| !seed_ids.contains(&feature_row.item_id) && feature_row.embedding.is_some())
            .collect();

        // Score candidates by average similarity to all seeds
        let mut results: Vec<ScoredItem> = Vec::new();

        for cand_feature in candidates_features {
            let cand_emb: Vec<f32> = cand_feature.embedding
                .and_then(|v| serde_json::from_value(serde_json::Value::from(v)).ok())
                .unwrap_or_else(|| vec![0.0; 128]);

            let avg_sim: f32 = seed_embeddings.iter()
                .map(|seed_emb| {
                    match params.method.as_str() {
                        "cosine" => cosine_similarity(seed_emb, &cand_emb),
                        "dot_product" => dot_product(seed_emb, &cand_emb),
                        _ => cosine_similarity(seed_emb, &cand_emb),
                    }
                })
                .sum::<f32>() / seed_embeddings.len().max(1) as f32;

            results.push(ScoredItem::new(
                cand_feature.item_id,
                avg_sim,
                json!({
                    "similarity_method": params.method,
                    "inference_engine": "native",
                }),
            ));
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(params.top_k);

        Ok(results)
    }
}



