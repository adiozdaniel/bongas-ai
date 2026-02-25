//! Engine layer — central coordination, execution, and intelligence.

pub mod engine;
pub mod execution_manager;
pub mod scenarios_manager;
pub mod suggestions_manager;
pub mod workers_manager;
pub mod staging_manager;
pub mod staleness_engine;
pub mod scenario_factory;
pub mod predictive_warmer;
pub mod analytics_sidecar;
pub mod hive_mind;
pub mod strategy_resolver;
pub mod context;
pub mod config;
pub mod runtime;

pub use engine::{BongasEngine, ScenarioDefinition, RecommendationItem, ScenarioExecutionStats, SecurityStatus};
pub use scenarios_manager::ScenariosManager;
pub use execution_manager::ExecutionManager;
pub use runtime::BongasRuntime;
