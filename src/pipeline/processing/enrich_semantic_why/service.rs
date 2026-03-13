use async_trait::async_trait;
use anyhow::Result;
use serde_json::{Value as JsonValue, json};
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;

/// Processing stage that provides human-readable reasoning for recommendations.
pub struct EnrichSemanticWhyStage;

#[async_trait]
impl PipelineStage for EnrichSemanticWhyStage {
    fn name(&self) -> &str {
        "enrich_semantic_why"
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
        let profile_id = match &context.profile_id {
            Some(pid) => pid,
            None => return Ok(input), // No persona context, no personalized reasoning
        };

        if input.is_empty() {
            return Ok(input);
        }

        // 1. Try to fetch pre-computed reasons from Redis (Batch)
        let keys: Vec<String> = input.iter()
            .map(|item| format!("reason:{}:{}", profile_id, item.item_id))
            .collect();

        let cached_reasons: Vec<Option<String>> = if let Some(mut conn) = context.cache_manager.l2_connection() {
            redis::AsyncCommands::mget(&mut conn, &keys).await.unwrap_or_else(|_| vec![None; keys.len()])
        } else {
            vec![None; keys.len()]
        };

        // 2. Map cached reasons or apply heuristic fallback
        let mut items_needing_fallback = Vec::new();
        for (i, reason_opt) in cached_reasons.into_iter().enumerate() {
            if let Some(reason) = reason_opt {
                input[i].reasoning.push(reason);
            } else {
                items_needing_fallback.push(i);
            }
        }

        // 3. Heuristic Fallback: Match Profile Genre Affinity to Item Tags
        if !items_needing_fallback.is_empty() {
            let profile_features = context.item_feature_service.get_profile_features(profile_id).await?;
            
            if let Some(profile) = profile_features {
                let item_ids: Vec<i32> = items_needing_fallback.iter().map(|&idx| input[idx].item_id).collect();
                let item_features = context.item_feature_service.get_item_features_batch(&item_ids).await?;

                let empty_json = json!({});
                let empty_array = json!([]);

                for idx in items_needing_fallback {
                    let item_id = input[idx].item_id;
                    if let Some(item) = item_features.get(&item_id) {
                        let matched_genre = find_genre_match(
                            profile.genre_affinity.as_ref().unwrap_or(&empty_json), 
                            item.tags.as_ref().unwrap_or(&empty_array)
                        );
                        let reason = match matched_genre {
                            Some(genre) => format!("Matches your interest in {}", genre),
                            None => "Recommended for you".to_string(),
                        };
                        input[idx].reasoning.push(reason);
                    }
                }
            } else {
                // Generic fallback if profile not found
                for idx in items_needing_fallback {
                    input[idx].reasoning.push("Recommended for you".to_string());
                }
            }
        }

        Ok(input)
    }
}

/// Simple heuristic to find a matching genre between profile affinities and item tags.
fn find_genre_match(affinities: &JsonValue, tags: &JsonValue) -> Option<String> {
    let aff_obj = affinities.as_object()?;
    let tag_arr = tags.as_array()?;

    for tag in tag_arr {
        if let Some(tag_str) = tag.as_str() {
            if aff_obj.contains_key(tag_str) {
                return Some(tag_str.to_string());
            }
        }
    }
    None
}
