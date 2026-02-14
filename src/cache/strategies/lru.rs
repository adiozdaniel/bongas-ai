  //! L1 in-memory LRU cache with size-based eviction.

  use crate::cache::traits::{CacheStrategy, CacheTier};
  use crate::cache::metrics::CacheMetrics;
  use async_trait::async_trait;
  use lru::LruCache as LruMap;
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;
  use std::time::{Duration, Instant};
  use tokio::sync::RwLock;
  use anyhow::Result;

  /// Entry in the LRU cache with expiration.
  #[derive(Clone)]
  struct CacheEntry {
      value: Vec<u8>,
      expires_at: Instant,
  }

  impl CacheEntry {
      fn is_expired(&self) -> bool {
          Instant::now() >= self.expires_at
      }
  }

  /// L1 in-memory LRU cache.
  pub struct LruCache {
      cache: Arc<RwLock<LruMap<String, CacheEntry>>>,
      metrics: Arc<CacheMetrics>,
  }

  impl LruCache {
      pub fn new(capacity: usize, metrics: Arc<CacheMetrics>) -> Self {
          Self {
              cache: Arc::new(RwLock::new(LruMap::new(capacity.try_into().unwrap()))),
              metrics,
          }
      }
  }

  #[async_trait]
  impl CacheStrategy for LruCache {
      async fn get<T>(&self, key: &str) -> Result<Option<T>>
      where
          T: for<'de> Deserialize<'de> + Send,
      {
          let mut cache = self.cache.write().await;

          if let Some(entry) = cache.get(key) {
              if entry.is_expired() {
                  cache.pop(key);
                  self.metrics.record_l1_miss();
                  return Ok(None);
              }

              self.metrics.record_l1_hit();
              let value: T = serde_json::from_slice(&entry.value)?;
              return Ok(Some(value));
          }

          self.metrics.record_l1_miss();
          Ok(None)
      }

      async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>
      where
          T: Serialize + Send + Sync,
      {
          let serialized = serde_json::to_vec(value)?;
          let entry = CacheEntry {
              value: serialized,
              expires_at: Instant::now() + ttl,
          };

          let mut cache = self.cache.write().await;
          if cache.put(key.to_string(), entry).is_some() {
              self.metrics.record_eviction();
          }

          Ok(())
      }

      async fn delete(&self, key: &str) -> Result<()> {
          let mut cache = self.cache.write().await;
          cache.pop(key);
          Ok(())
      }

      async fn exists(&self, key: &str) -> Result<bool> {
          let cache = self.cache.read().await;
          Ok(cache.contains(key))
      }

      async fn clear(&self) -> Result<()> {
          let mut cache = self.cache.write().await;
          cache.clear();
          Ok(())
      }

      fn name(&self) -> &'static str {
          "lru"
      }

      fn tier(&self) -> CacheTier {
          CacheTier::L1
      }
  }

