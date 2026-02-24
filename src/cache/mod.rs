//! Netflix-grade multi-tier cache module.
//!
//! Provides:
//! - L1: In-memory LRU cache
//! - L2: Redis cache with circuit breaker
//! - Metrics collection
//! - Cache warming

pub mod config;
pub mod manager;
pub mod metrics;
pub mod strategies;
pub mod traits;
pub mod warming;
pub mod hot_registry;

pub use config::CacheConfig;
pub use manager::CacheManager;
pub use metrics::{CacheMetrics, CacheMetricsSnapshot};
pub use strategies::{LruCache, NoOpCache, RedisCache, CacheLayer};
pub use traits::{CacheStrategy, CacheTier};
pub use warming::CacheWarmer;
pub use hot_registry::{HotRegistry, HotItem};
