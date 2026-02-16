  //! Netflix-grade multi-tier cache module.
  //!
  //! Provides:
  //! - L1: In-memory LRU cache
  //! - L2: Redis cache with circuit breaker
  //! - Metrics collection
  //! - Cache warming
  //!
  //! # Architecture
  //! ```text
  //! CacheManager
  //!   ├─> L1 (LRU) ────> Hit: Return
  //!   │                  Miss: ↓
  //!   ├─> L2 (Redis) ──> Hit: Backfill L1, Return
  //!   │                  Miss: ↓
  //!   └─> Compute ─────> Store in L2 + L1, Return
  //! ```

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
  pub use strategies::{LruCache, NoOpCache, RedisCache};
  pub use traits::{CacheStrategy, CacheTier};
  pub use warming::CacheWarmer;
  pub use hot_registry::{HotRegistrySafe, HotItem};
