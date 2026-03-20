//! Coordination domain — orchestrates the three pillars of the Bongas engine.

pub mod service;
pub mod builder;

pub use service::{BongasEngine, EngineComponents, ScenarioDefinition, ScenarioExecutionStats, RecommendationItem};
pub use builder::DiscoverySymphony;
