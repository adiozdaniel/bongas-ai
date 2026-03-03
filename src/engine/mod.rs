//! Engine layer — central coordination, execution, and intelligence.
//! The Bongas-AI Symphony: Functional pillars for Enterprise-Grade Discovery.

pub mod execution;
pub mod governance;
pub mod intelligence;
pub mod coordination;
pub mod config;

// Re-export key types for public API stability and convenience
pub use coordination::service::{BongasEngine, ScenarioDefinition, RecommendationItem, ScenarioExecutionStats, SecurityStatus};
pub use execution::core::execution_manager::ExecutionManager;
pub use execution::runtime::runtime::BongasRuntime;
pub use governance::factory::scenarios_manager::ScenariosManager;
