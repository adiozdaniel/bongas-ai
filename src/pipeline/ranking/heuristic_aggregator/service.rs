use async_trait::async_trait;
use anyhow::{Result, Context};
use serde_json::{Value as JsonValue, json};
use serde::Deserialize;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind, CompactMetadata};
use crate::pipeline::context::service::ExecutionContext;

#[derive(Deserialize)]
struct Params {
    /// List of heuristics to aggregate (currently supporting: "popularity", "recency", "affinity")
    features: Vec<String>,
}

/// Aggregates multiple heuristic scores into a single feature vector for ML ranking.
pub struct HeuristicAggregatorStage;

#[async_trait]
impl PipelineStage for HeuristicAggregatorStage {
    fn name(&self) -> &str {
        "heuristic_aggregator"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    fn output_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        let params: Params = serde_json::from_value(params.clone())
            .context("Failed to parse heuristic_aggregator params")?;

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let item_ids: Vec<i32> = input.iter().map(|item| item.item_id).collect();
        
        // Fetch item features once
        let item_features_map = context.item_feature_service
            .get_item_features_batch(&item_ids)
            .await?;

        // Mock User affinity for now (in real system, fetch from context.feature_store)
        let user_affinity = 0.5; 

        let processed: Vec<ScoredItem> = input.into_iter().map(|mut item| {
            let mut feature_vector = Vec::with_capacity(params.features.len());
            
            if let Some(features) = item_features_map.get(&item.item_id) {
                for feat_name in &params.features {
                    let val = match feat_name.as_str() {
                        "popularity" => features.popularity_score.unwrap_or(0.0),
                        "trending" => features.trending_score,
                        "recency" => {
                            // Simple recency calculation
                            features.release_year.map(|y| (y as f32 - 2000.0) / 25.0).unwrap_or(0.0)
                        },
                        "affinity" => user_affinity,
                        _ => 0.0,
                    };
                    feature_vector.push(val);
                }
            } else {
                feature_vector.extend(vec![0.0; params.features.len()]);
            }

            // Pack into metadata
            item.metadata["heuristic_vector"] = json!(feature_vector);

            // Phase 6: Zero-Copy Fast Path
            let compact = CompactMetadata {
                features: feature_vector,
                flags: 0,
                category_id: 0,
            };

            if let Ok(bytes) = rkyv::to_bytes::<_, 256>(&compact) {
                item.fast_metadata = Some(bytes.to_vec());
            }

            item
        }).collect();

        Ok(processed)
    }
}
