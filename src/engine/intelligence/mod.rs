//! Intelligence Pillar: AI-driven insights and self-optimization.

pub mod ai;
pub mod monitoring;
pub mod workers;
pub mod identity;
pub mod pillar;

pub use ai::suggestions_manager::service::SuggestionsManager;
pub use ai::hive_mind::service::HiveMindConnector;
pub use monitoring::analytics_sidecar::service::AnalyticsSidecar;
pub use monitoring::staleness_engine::service::StalenessEngine;
pub use workers::workers_manager::service::WorkersManager;
pub use identity::service::IdentityStitcher;
pub use pillar::service::IntelligencePillar;
