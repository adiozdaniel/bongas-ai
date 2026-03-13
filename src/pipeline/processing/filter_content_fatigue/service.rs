use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use redis::AsyncCommands;
use tracing::debug;

/// Processing stage that filters or penalizes over-exposed items to prevent user fatigue.
pub struct FilterContentFatigueStage;

#[async_trait]
impl PipelineStage for FilterContentFatigueStage {
    fn name(&self) -> &str {
        "filter_content_fatigue"
    }

    fn input_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn output_type(&self) -> StageDataKind { StageDataKind::ScoredItems }
    fn can_parallelize(&self) -> bool { true }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // 1. Guard: Check if fatigue is enabled and profile is available
        if !context.ml_config.fatigue_enabled {
            return Ok(input);
        }

        let profile_id = match &context.profile_id {
            Some(pid) => pid,
            None => return Ok(input), // Anonymous users don't have persistent fatigue tracking yet
        };

        if input.is_empty() {
            return Ok(input);
        }

        // 2. Batch Fetch Fatigue Counters (Redis MGET)
        let keys: Vec<String> = input.iter()
            .map(|item| format!("seen:{}:{}", profile_id, item.item_id))
            .collect();

        let counts: Vec<Option<u32>> = if let Some(mut conn) = context.cache_manager.l2_connection() {
            conn.mget(&keys).await.unwrap_or_else(|_| vec![None; keys.len()])
        } else {
            vec![None; keys.len()]
        };

        // 3. Apply Filtering and Penalties
        let max_exposures = context.ml_config.fatigue_max_exposures;
        let penalty_factor = context.ml_config.fatigue_penalty_factor;

        let mut filtered_input = Vec::with_capacity(input.len());

        for (item, count_opt) in input.into_iter().zip(counts.into_iter()) {
            let count = count_opt.unwrap_or(0);

            if count >= max_exposures {
                // Drop the item
                debug!(item_id = item.item_id, count, "Fatigue: Item dropped due to over-exposure");
                continue;
            }

            let mut final_item = item;
            if count > 0 {
                // Apply exponential penalty: score = score * (penalty_factor ^ count)
                let multiplier = penalty_factor.powi(count as i32);
                final_item.score *= multiplier;
                final_item.reasoning.push(format!("Fatigue Penalty: {} previous exposures", count));
            }

            filtered_input.push(final_item);
        }

        Ok(filtered_input)
    }
}
