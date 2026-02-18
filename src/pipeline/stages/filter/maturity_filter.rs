use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem};
use crate::pipeline::context::ExecutionContext;
use tracing::debug;

/// Phase 11: Maturity Filter Stage
/// 
/// Filters content based on the user's maturity rating (X-MATURITY-RATING header).
/// Standard Ratings: GE (0+), PG (13+), 16 (16+), 18 (18+)
pub struct MaturityFilterStage;

impl MaturityFilterStage {
    fn rating_to_age(rating: &str) -> i32 {
        match rating.to_uppercase().as_str() {
            "GE" => 0,
            "PG" => 13,
            "16" => 16,
            "18" => 18,
            _ => 18, // Default to strictest for unknown
        }
    }
}

#[async_trait]
impl PipelineStage for MaturityFilterStage {
    fn name(&self) -> &str {
        "maturity_filter"
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // Extract user maturity from context
        // Priority: 1. Explicit context.maturity_rating, 2. Experiment override (legacy), 3. Default "GE"
        let user_rating_str = context.maturity_rating.as_deref()
            .or_else(|| context.experiment_overrides.get("user_maturity_rating").and_then(|v| v.as_str()))
            .unwrap_or("GE");
        
        let user_age_limit = Self::rating_to_age(user_rating_str);

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let before_count = input.len();
        
        let filtered: Vec<ScoredItem> = input.into_iter()
            .filter(|item| {
                let item_rating_str = item.metadata.get("age_rating")
                    .and_then(|v| v.as_str())
                    .unwrap_or("18"); // If item has no rating, assume it's for adults
                
                let item_age_req = Self::rating_to_age(item_rating_str);
                
                item_age_req <= user_age_limit
            })
            .collect();

        debug!(
            request_id = %context.request_id,
            user_maturity = %user_rating_str,
            filtered = before_count - filtered.len(),
            "Maturity filtering complete"
        );

        Ok(filtered)
    }
}
