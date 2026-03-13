use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind, MaturityRating};
use crate::pipeline::context::service::ExecutionContext;
use crate::engine::intelligence::ai::hive_mind::service::PulseClassification;
use std::str::FromStr;

/// Ranking stage that boosts content matching regional semantic events.
pub struct BoostHyperLocalPulseStage;

#[async_trait]
impl PipelineStage for BoostHyperLocalPulseStage {
    fn name(&self) -> &str {
        "boost_hyper_local_pulse"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        mut input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // 1. Guard: Check if location is available
        let location = match &context.location {
            Some(loc) => loc.to_lowercase(),
            None => return Ok(input), // Fail open if no location
        };

        // 2. Fetch active Physical Event from Redis
        let pulse_key = format!("pulse:{}", location);
        let classification: Option<PulseClassification> = context.cache_manager.get(
            &pulse_key, 
            "boost_hyper_local_pulse", 
            context.user_id, 
            context.profile_id.as_deref()
        ).await?;

        let pulse = match classification {
            Some(p) => p,
            None => return Ok(input), // No active regional pulse
        };

        // 3. Guardrail: Maturity Check
        if let Some(ref user_maturity_str) = context.maturity_rating {
            let user_maturity = MaturityRating::from_str(user_maturity_str).unwrap_or(MaturityRating::M18);
            
            // If the event is "Natural Disaster" and user is Kids (GE), abort boost to avoid trauma
            if pulse.theme == "Natural Disaster" && user_maturity == MaturityRating::GE {
                return Ok(input);
            }
        }

        // 4. Batch Fetch Item Features (Embeddings)
        let item_ids: Vec<i32> = input.iter().map(|i| i.item_id).collect();
        let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

        // 5. Apply Boost based on Cosine Similarity
        for scored_item in &mut input {
            if let Some(features) = item_features.get(&scored_item.item_id) {
                if let Some(ref item_emb) = features.embedding {
                    let similarity = cosine_similarity(item_emb, &pulse.semantic_vector);

                    if similarity > 0.7 {
                        let boost_factor = 1.3;
                        scored_item.score *= boost_factor;
                        scored_item.reasoning.push(format!(
                            "Regional Boost: High community interest in {} for this profile",
                            pulse.theme
                        ));
                    }
                }
            }
        }

        Ok(input)
    }
}

/// Simple cosine similarity helper
fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    if v1.len() != v2.len() || v1.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let norm1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let norm2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    
    if norm1 == 0.0 || norm2 == 0.0 {
        return 0.0;
    }
    dot_product / (norm1 * norm2)
}
