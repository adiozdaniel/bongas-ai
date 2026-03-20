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

      /// Push a value to a list (for user history/Ghost Cache).
      async fn push_to_list(&self, key: &str, value: String, max_len: usize) -> Result<()>;

      /// Get all values from a list (for user history/Ghost Cache).
      async fn get_list(&self, key: &str) -> Result<Vec<String>>;

      /// Set a raw string value (bypass bincode).
      async fn set_raw(&self, key: &str, value: String, ttl: Duration) -> Result<()>;

      /// Get a raw string value (bypass bincode).
      async fn get_raw(&self, key: &str) -> Result<Option<String>>;

      /// Delete a value from cache.
      async fn delete(&self, key: &str) -> Result<()>;

      /// Delete multiple keys matching a pattern (wildcards supported).
      async fn delete_pattern(&self, pattern: &str) -> Result<()>;

      /// Check if key exists.
      async fn exists(&self, key: &str) -> Result<bool>;

      /// Clear all entries (for testing/warmup).
      async fn clear(&self) -> Result<()>;

      /// Get cache name for metrics.
      fn name(&self) -> &'static str;

      /// Get cache tier (L1, L2, etc).
      fn tier(&self) -> CacheTier;

      /// Close the cache gracefully.
      async fn close(&self) -> Result<()> {
          Ok(())
      }
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

