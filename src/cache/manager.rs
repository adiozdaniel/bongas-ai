  //! Netflix-grade cache manager with L1 (LRU) + L2 (Redis) composite pattern.

  use crate::cache::config::CacheConfig;
  use crate::cache::metrics::{CacheMetrics, CacheMetricsSnapshot};
  use crate::cache::strategies::{LruCache, RedisCache, CacheLayer};
  use crate::cache::traits::CacheStrategy;
  use anyhow::Result;
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;

  /// Netflix-grade cache manager with multi-tier caching.
  ///
  /// Architecture:
  /// - L1: In-memory LRU (fast, local)
  /// - L2: Redis (distributed, circuit breaker protected)
  /// - Fallback: Compute function
      pub struct CacheManager {
          l1: Option<CacheLayer>,
          l2: Option<CacheLayer>,
          config: CacheConfig,
          metrics: Arc<CacheMetrics>,
      }
  impl CacheManager {
      /// Create a new cache manager.
      pub async fn new(redis_url: &str, config: CacheConfig) -> Result<Self> {
          let metrics = Arc::new(CacheMetrics::new());

          let l1: Option<CacheLayer> = if config.l1_enabled {
              Some(CacheLayer::Lru(LruCache::new(
                  config.l1_max_entries,
                  Arc::clone(&metrics),
              )))
          } else {
              None
          };

          let l2: Option<CacheLayer> = if config.l2_enabled {
              match RedisCache::new(redis_url, Arc::clone(&metrics)).await {
                  Ok(cache) => Some(CacheLayer::Redis(cache)),
                  Err(e) => {
                      tracing::warn!("Failed to initialize Redis cache: {}", e);
                      None
                  }
              }
          } else {
              None
          };

          Ok(Self {
              l1,
              l2,
              config,
              metrics,
          })
      }

      /// Get a value from cache with fallback to compute function.
      pub async fn get_or_compute<T, F, Fut>(
          &self,
          key: &str,
          compute: F,
      ) -> Result<T>
      where
          T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
          F: FnOnce() -> Fut + Send,
          Fut: std::future::Future<Output = Result<T>> + Send,
      {
          // Try L1 cache
          if let Some(ref l1) = self.l1 {
              if let Some(value) = l1.get::<T>(key).await? {
                  return Ok(value);
              }
          }

          // Try L2 cache
          if let Some(ref l2) = self.l2 {
              if let Some(value) = l2.get::<T>(key).await? {
                  // Backfill L1
                  if let Some(ref l1) = self.l1 {
                      let _ = l1.set(key, &value, self.config.l1_ttl).await;
                  }
                  return Ok(value);
              }
          }

          // Compute value
          let value = compute().await?;

          // Store in caches
          if let Some(l2) = &self.l2 {
              let _ = l2.set(key, &value, self.config.l2_ttl).await;
          }
          if let Some(l1) = &self.l1 {
              let _ = l1.set(key, &value, self.config.l1_ttl).await;
          }

          Ok(value)
      }

      /// Get a value from cache only (no compute).
      pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
      where
          T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone,
      {
          // Try L1
          if let Some(ref l1) = self.l1 {
              if let Some(value) = l1.get::<T>(key).await? {
                  return Ok(Some(value));
              }
          }

          // Try L2
          if let Some(ref l2) = self.l2 {
              if let Some(value) = l2.get::<T>(key).await? {
                  // Backfill L1
                  if let Some(ref l1) = self.l1 {
                      let _ = l1.set(key, &value, self.config.l1_ttl).await;
                  }
                  return Ok(Some(value));
              }
          }

          Ok(None)
      }

      /// Set a value in all cache tiers.
      pub async fn set<T>(&self, key: &str, value: &T) -> Result<()>
      where
          T: Serialize + Send + Sync,
      {
          if let Some(ref l2) = self.l2 {
              let _ = l2.set(key, value, self.config.l2_ttl).await;
          }
          if let Some(ref l1) = self.l1 {
              let _ = l1.set(key, value, self.config.l1_ttl).await;
          }
          Ok(())
      }

      /// Delete a value from all cache tiers.
      pub async fn delete(&self, key: &str) -> Result<()> {
          if let Some(ref l1) = self.l1 {
              let _ = l1.delete(key).await;
          }
          if let Some(ref l2) = self.l2 {
              let _ = l2.delete(key).await;
          }
          Ok(())
      }

      /// Get cache metrics snapshot.
      pub fn metrics(&self) -> CacheMetricsSnapshot {
          self.metrics.snapshot()
      }

      /// Clear all caches.
      pub async fn clear(&self) -> Result<()> {
          if let Some(ref l1) = self.l1 {
              l1.clear().await?;
          }
          if let Some(ref l2) = self.l2 {
              l2.clear().await?;
          }
          Ok(())
      }
  }

