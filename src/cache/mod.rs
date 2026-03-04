//! Netflix-grade multi-tier cache module.
//!
//! Organized into functional sub-modules for configuration, management, 
//! metrics, strategies, and warming.

pub mod config;
pub mod manager;
pub mod metrics;
pub mod strategies;
pub mod traits;
pub mod warming;
pub mod hot_registry;

// Re-export core types for external consumption
pub use config::models::CacheConfig;
pub use manager::service::CacheManager;
pub use metrics::service::{CacheMetrics, CacheMetricsSnapshot};
pub use strategies::lru::service::LruCache;
pub use strategies::redis::service::RedisCache;
pub use strategies::noop::service::NoOpCache;
pub use strategies::service::CacheLayer;
pub use traits::models::{CacheStrategy, CacheTier};
pub use warming::service::CacheWarmer;
pub use hot_registry::service::{HotRegistry, HotItem};
