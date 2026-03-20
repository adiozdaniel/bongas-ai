//! L1 in-memory LRU cache with count-based eviction and sharding.

  use crate::cache::{CacheStrategy, CacheTier};
  use crate::cache::CacheMetrics;
  use async_trait::async_trait;
  use lru::LruCache as LruMap;
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;
  use std::time::{Duration, Instant};
  use tokio::sync::RwLock;
  use anyhow::Result;
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  const SHARD_COUNT: usize = 64;

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

  /// L1 in-memory LRU cache with sharding for high concurrency.
  pub struct LruCache {
      shards: Vec<Arc<RwLock<LruMap<String, CacheEntry>>>>,
      metrics: Arc<CacheMetrics>,
  }

  impl LruCache {
      pub fn new(capacity: usize, metrics: Arc<CacheMetrics>) -> Self {
          let shard_capacity = (capacity / SHARD_COUNT).max(1);
          let mut shards = Vec::with_capacity(SHARD_COUNT);
          for _ in 0..SHARD_COUNT {
              shards.push(Arc::new(RwLock::new(LruMap::new(shard_capacity.try_into().unwrap()))));
          }

          Self {
              shards,
              metrics,
          }
      }

      fn get_shard_index(&self, key: &str) -> usize {
          let mut hasher = DefaultHasher::new();
          key.hash(&mut hasher);
          (hasher.finish() as usize) % SHARD_COUNT
      }
  }

  #[async_trait]
  impl CacheStrategy for LruCache {
      async fn get<T>(&self, key: &str) -> Result<Option<T>>
      where
          T: for<'de> Deserialize<'de> + Send,
      {
          let shard_idx = self.get_shard_index(key);
          
          // Fix #30: Optimized lock pattern - try read lock first
          {
              let cache = self.shards[shard_idx].read().await;
              if let Some(entry) = cache.peek(key) {
                  if entry.is_expired() {
                      // Need write lock to remove
                  } else {
                      // Valid hit - but we still need write lock to update LRU order via cache.get()
                  }
              } else {
                  self.metrics.record_l1_miss();
                  return Ok(None);
              }
          }

          // LRU get requires mutable access to update access order
          let mut cache = self.shards[shard_idx].write().await;

          if let Some(entry) = cache.get(key) {
              if entry.is_expired() {
                  cache.pop(key);
                  self.metrics.record_l1_miss();
                  return Ok(None);
              }

              self.metrics.record_l1_hit();
              // Fix #31: Use bincode for parity with L2
              let value: T = bincode::deserialize(&entry.value)?;
              return Ok(Some(value));
          }

          self.metrics.record_l1_miss();
          Ok(None)
      }

      async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>
      where
          T: Serialize + Send + Sync,
      {
          // Fix #31: Use bincode for parity with L2
          let serialized = bincode::serialize(value)?;
          let entry = CacheEntry {
              value: serialized,
              expires_at: Instant::now() + ttl,
          };

          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          // put returns the old value if it existed, or None if it was an eviction or replacement
          if cache.put(key.to_string(), entry).is_none() && cache.len() == cache.cap().get() {
              self.metrics.record_eviction();
          }

          Ok(())
      }

      async fn push_to_list(&self, key: &str, value: String, max_len: usize) -> Result<()> {
          let mut list = self.get_list(key).await?;
          list.insert(0, value);
          list.truncate(max_len);
          
          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          
          let serialized = bincode::serialize(&list)?;
          let entry = CacheEntry {
              value: serialized,
              expires_at: Instant::now() + Duration::from_secs(3600), // Default 1h for lists in L1
          };
          
          cache.put(key.to_string(), entry);
          Ok(())
      }

      async fn get_list(&self, key: &str) -> Result<Vec<String>> {
          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          
          if let Some(entry) = cache.get(key) {
              if !entry.is_expired() {
                  let list: Vec<String> = bincode::deserialize(&entry.value).unwrap_or_default();
                  return Ok(list);
              } else {
                  cache.pop(key);
              }
          }
          
          Ok(Vec::new())
      }

      async fn set_raw(&self, key: &str, value: String, ttl: Duration) -> Result<()> {
          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          
          let entry = CacheEntry {
              value: value.into_bytes(),
              expires_at: Instant::now() + ttl,
          };
          
          cache.put(key.to_string(), entry);
          Ok(())
      }

      async fn get_raw(&self, key: &str) -> Result<Option<String>> {
          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          
          if let Some(entry) = cache.get(key) {
              if !entry.is_expired() {
                  return Ok(Some(String::from_utf8_lossy(&entry.value).to_string()));
              } else {
                  cache.pop(key);
              }
          }
          
          Ok(None)
      }

      async fn delete(&self, key: &str) -> Result<()> {
          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          cache.pop(key);
          Ok(())
      }

      async fn delete_pattern(&self, pattern: &str) -> Result<()> {
          // Translate glob-like pattern to regex for more flexible matching
          // Supports: * (any chars), ? (any single char)
          let regex_pattern = format!("^{}$", pattern.replace(".", "\\.").replace("*", ".*").replace("?", "."));
          let re = regex::Regex::new(&regex_pattern)?;

          for shard in &self.shards {
              let mut cache = shard.write().await;
              // This is slow (O(N) per shard) but necessary for pattern invalidation in L1
              let keys_to_remove: Vec<String> = cache.iter()
                  .filter(|(k, _)| re.is_match(k))
                  .map(|(k, _)| k.clone())
                  .collect();
              
              for k in keys_to_remove {
                  cache.pop(&k);
              }
          }
          Ok(())
      }

      async fn exists(&self, key: &str) -> Result<bool> {
          let shard_idx = self.get_shard_index(key);
          let mut cache = self.shards[shard_idx].write().await;
          
          // Fix #70: exists() must check expiration
          if let Some(entry) = cache.get(key) {
              if entry.is_expired() {
                  cache.pop(key);
                  return Ok(false);
              }
              Ok(true)
          } else {
              Ok(false)
          }
      }

      async fn clear(&self) -> Result<()> {
          for shard in &self.shards {
              let mut cache = shard.write().await;
              cache.clear();
          }
          Ok(())
      }

      fn name(&self) -> &'static str {
          "lru_sharded"
      }

      fn tier(&self) -> CacheTier {
          CacheTier::L1
      }
  }
