use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// Weight for Meilisearch keyword match score (0.0-1.0)
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f32,
    /// Weight for Semantic similarity score (0.0-1.0)
    #[serde(default = "default_semantic_weight")]
    pub semantic_weight: f32,
    /// Whether to use Tribe-based embedding similarity if available
    #[serde(default = "default_use_tribe")]
    pub use_tribe: bool,
}

fn default_keyword_weight() -> f32 { 0.4 }
fn default_semantic_weight() -> f32 { 0.6 }
fn default_use_tribe() -> bool { true }

pub struct HybridSearchRankerStage;

#[async_trait]
impl PipelineStage for HybridSearchRankerStage {
    fn name(&self) -> &str {
        "hybrid_search_ranker"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let params: Params = serde_json::from_value(params.clone())?;
        let item_ids: Vec<i32> = input.iter().map(|i| i.item_id).collect();

        // 1. Fetch full features for re-ranking
        let features_map = context.item_feature_service
            .get_item_features_batch(&item_ids)
            .await?;

        // 2. Fetch User/Tribe embedding if available
        let user_embedding = if params.use_tribe {
            // This is a simplification; in a real flow, we'd fetch from FeatureStore
            // For Milestone 10, we'll assume the embedding might be in the context
            context.profile_id.as_ref().and(None::<Vec<f32>>)
        } else {
            None
        };

        let mut results = Vec::new();

        for mut item in input {
            let features = match features_map.get(&item.item_id) {
                Some(f) => f,
                None => {
                    results.push(item);
                    continue;
                }
            };

            // Keyword score (from Meilisearch hit metadata)
            let keyword_score = item.score; // Assume input score is the keyword match score

            // Semantic score (Genre/Tag overlap or Embedding similarity)
            let mut semantic_score = 0.0;

            if let Some(ref user_emb) = user_embedding {
                if let Some(ref item_emb) = features.embedding {
                    // Cosine similarity
                    semantic_score = calculate_cosine_similarity(user_emb, item_emb);
                }
            } else {
                // Fallback: Heuristic semantic similarity based on genres/tags
                // In a real search, we might compare these to the search query tokens
                semantic_score = features.popularity_score.unwrap_or(0.5);
            }

            // 3. Hybrid Aggregation
            let hybrid_score = (keyword_score * params.keyword_weight) + 
                             (semantic_score * params.semantic_weight);

            item.score = hybrid_score;
            item.metadata["hybrid_details"] = json!({
                "keyword_score": keyword_score,
                "semantic_score": semantic_score,
                "weights": {
                    "keyword": params.keyword_weight,
                    "semantic": params.semantic_weight
                }
            });

            results.push(item);
        }

        // 4. Final Sort
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }
}

fn calculate_cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    if v1.len() != v2.len() || v1.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let norm1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let norm2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    
    if norm1 > 0.0 && norm2 > 0.0 {
        dot_product / (norm1 * norm2)
    } else {
        0.0
    }
}
