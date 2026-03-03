//! Execution Pillar: The high-performance discovery path.

pub mod core;
pub mod cache;
pub mod runtime;

use std::sync::Arc;
pub use core::execution_manager::ExecutionManager;
pub use crate::engine::governance::strategy::resolver::StrategyResolver;
pub use cache::predictive_warmer::PredictiveWarmer;
pub use cache::staging_manager::StagingManager;
pub use crate::pipeline::context::ExecutionContext;
pub use runtime::runtime::BongasRuntime;

/// ⚡ THE STAGE: High-performance discovery execution.
pub struct ExecutionPillar {
    pub manager: Arc<ExecutionManager>,
    pub resolver: Arc<StrategyResolver>,
    pub warmer: Arc<PredictiveWarmer>,
    pub staging: Arc<StagingManager>,
}

impl ExecutionPillar {
    pub fn new(
        manager: Arc<ExecutionManager>,
        resolver: Arc<StrategyResolver>,
        warmer: Arc<PredictiveWarmer>,
        staging: Arc<StagingManager>,
    ) -> Self {
        Self { manager, resolver, warmer, staging }
    }
}
