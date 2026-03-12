//! Execution Pillar: The high-performance discovery path.

pub mod core;
pub mod cache;
pub mod runtime;
pub mod pillar;

pub use core::execution_manager::service::ExecutionManager;
pub use crate::engine::governance::strategy::resolver::service::StrategyResolver;
pub use cache::predictive_warmer::service::PredictiveWarmer;
pub use cache::staging_manager::service::StagingManager;
pub use crate::pipeline::context::service::ExecutionContext;
pub use runtime::service::BongasRuntime;
pub use pillar::service::ExecutionPillar;
