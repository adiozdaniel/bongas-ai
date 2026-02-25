use async_trait::async_trait;
use anyhow::Result;
use crate::pipeline::ScoredItem;
use crate::pipeline::context::ExecutionContext;

/// Core trait for different scenario execution strategies.
///
/// Scenarios can be implemented as hardcoded Rust logic or
/// dynamic JSONB-defined pipelines stored in the database.
#[async_trait]
pub trait ScenarioStrategy: Send + Sync {
    /// Strategy name (e.g., "dynamic_pipeline", "personalized_home")
    fn name(&self) -> &str;

    /// Scenario unique slug (e.g., "trending_now")
    fn slug(&self) -> &str;

    /// Execute the scenario logic and return scored items.
    async fn execute(
        &self,
        context: &ExecutionContext,
        user_id: Option<i32>,
    ) -> Result<Vec<ScoredItem>>;
}
