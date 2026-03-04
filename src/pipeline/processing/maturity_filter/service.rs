use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value as JsonValue;
use crate::pipeline::{PipelineStage, ScoredItem, MaturityRating, StageDataKind};
use crate::pipeline::context::service::ExecutionContext;
use tracing::debug;

/// Phase 11: Maturity Filter Stage
/// 
/// Filters content based on the user's maturity rating (X-MATURITY-RATING header).
/// Standard Ratings: GE (0+), PG (13+), 16 (16+), 18 (18+)
pub struct MaturityFilterStage;

#[async_trait]
impl PipelineStage for MaturityFilterStage {
    fn name(&self) -> &str {
        "maturity_filter"
    }

    fn input_type(&self) -> StageDataKind {
        StageDataKind::ScoredItems
    }

    async fn execute(
        &self,
        context: &ExecutionContext,
        _params: &JsonValue,
        input: Vec<ScoredItem>,
    ) -> Result<Vec<ScoredItem>> {
        // Extract user maturity from context
        let user_rating_str = context.maturity_rating.as_deref()
            .or_else(|| context.experiment_overrides.get("user_maturity_rating").and_then(|v| v.as_str()))
            .unwrap_or("GE");
        
        let user_rating = MaturityRating::from_str(user_rating_str);

        if input.is_empty() {
            return Ok(Vec::new());
        }

        let before_count = input.len();
        
        let filtered: Vec<ScoredItem> = input.into_iter()
            .filter_map(|mut item| {
                let item_rating_str = item.metadata.get("age_rating")
                    .and_then(|v| v.as_str())
                    .unwrap_or("18"); 
                
                let item_rating = MaturityRating::from_str(item_rating_str);
                
                if item_rating <= user_rating {
                    item.reasoning.push(format!("MaturityFilter: OK ({:?} <= {:?})", item_rating, user_rating));
                    Some(item)
                } else {
                    None
                }
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
