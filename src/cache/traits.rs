  //! Cache strategy trait for the Strategy Pattern.
  //!
  //! Defines the contract for all cache implementations (LRU, Redis, NoOp).

  use async_trait::async_trait;
  use serde::{Deserialize, Serialize};
  use std::time::Duration;
  use anyhow::Result;

  /// Cache strategy trait for pluggable cache implementations.
  ///
  /// Implementations include:
  /// - LRU: In-memory cache with size-based eviction
  /// - Redis: Distributed cache with circuit breaker protection
  /// - NoOp: No-op cache for testing
  #[async_trait]
  pub trait CacheStrategy: Send + Sync {
      /// Get a value from cache.
      async fn get<T>(&self, key: &str) -> Result<Option<T>>
      where
          T: for<'de> Deserialize<'de> + Send;

      /// Set a value in cache with TTL.
      async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>
      where
          T: Serialize + Send + Sync;

      /// Delete a value from cache.
      async fn delete(&self, key: &str) -> Result<()>;

      /// Check if key exists.
      async fn exists(&self, key: &str) -> Result<bool>;

      /// Clear all entries (for testing/warmup).
      async fn clear(&self) -> Result<()>;

      /// Get cache name for metrics.
      fn name(&self) -> &'static str;

      /// Get cache tier (L1, L2, etc).
      fn tier(&self) -> CacheTier;
  }

  /// Cache tier for metrics categorization.
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum CacheTier {
      L1,  // In-memory
      L2,  // Redis
      L3,  // Database/fallback
  }

  impl CacheTier {
      pub fn as_str(&self) -> &'static str {
          match self {
              CacheTier::L1 => "l1",
              CacheTier::L2 => "l2",
              CacheTier::L3 => "l3",
          }
      }
  }

