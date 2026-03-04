//! Database layer — resilient storage, connection pooling, and repositories.

pub mod metrics;
pub mod models;
pub mod pool;
pub mod repositories;

// Re-exports
pub use metrics::service::{DatabaseMetrics, DatabaseMetricsSnapshot};
pub use pool::service::{PoolStats, ResilientPool, ResilientPoolConfig};
pub use models::models::*;
pub use repositories::interaction_repository::service::InteractionRepository;
pub use repositories::item_feature_service::service::ItemFeatureService;
pub use repositories::model_repository::service::ModelRepository;
pub use repositories::cache_repository::service::CacheRepository;
pub use repositories::scenario_repository::service::ScenarioRepository;
pub use repositories::page_layout_repository::service::PageLayoutRepository;
pub use repositories::feature_repository::service::FeatureRepository;
pub use repositories::item_feature_service::models::*;
