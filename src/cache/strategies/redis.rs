//! L2 Redis cache with circuit breaker protection.

  use crate::cache::traits::{CacheStrategy, CacheTier};
  use crate::cache::metrics::CacheMetrics;
  use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId};
  use crate::circuit_breaker::observer::NoOpObserver;
  use crate::error::RedisError;
  use async_trait::async_trait;
  use redis::aio::ConnectionManager;
  use redis::{AsyncCommands, Client};
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;
  use std::time::Duration;
  use anyhow::{Result, Context};

  /// L2 Redis cache with circuit breaker.
  pub struct RedisCache {
      client: ConnectionManager,
      circuit_breaker: Arc<CircuitBreaker>,
      metrics: Arc<CacheMetrics>,
      prefix: String,
  }

  impl RedisCache {
      pub async fn new(
          redis_url: &str,
          metrics: Arc<CacheMetrics>,
      ) -> Result<Self> {
          let client = Client::open(redis_url)
              .context("Failed to create Redis client")?;
          let conn_manager = ConnectionManager::new(client).await
              .context("Failed to connect to Redis")?;

          // Create circuit breaker for Redis operations
          let cb_config = CircuitBreakerConfig::default();
          let circuit_breaker = Arc::new(CircuitBreaker::new(
              CircuitBreakerId::new("redis_cache"),
              cb_config,
              Arc::new(NoOpObserver),
          ));

          Ok(Self {
              client: conn_manager,
              circuit_breaker,
              metrics,
              // Fix #69: Namespace keys to avoid wiping other Redis data (like rate limits)
              prefix: "bongas:cache:".to_string(),
          })
      }

      fn prefixed_key(&self, key: &str) -> String {
          format!("{}{}", self.prefix, key)
      }
  }

  #[async_trait]
  impl CacheStrategy for RedisCache {
      async fn get<T>(&self, key: &str) -> Result<Option<T>>
      where
          T: for<'de> Deserialize<'de> + Send,
      {
          let mut conn = self.client.clone();
          let key_owned = self.prefixed_key(key);

          let result = self.circuit_breaker.call(|| async {
              let value: Option<Vec<u8>> = conn.get(&key_owned).await.map_err(RedisError::from)?;
              Ok::<_, RedisError>(value)
          }).await;

          match result {
              Ok(Some(bytes)) => {
                  self.metrics.record_l2_hit();
                  let value: T = bincode::deserialize(&bytes)?;
                  Ok(Some(value))
              }
              Ok(None) => {
                  self.metrics.record_l2_miss();
                  Ok(None)
              }
              Err(_) => {
                  self.metrics.record_error();
                  self.metrics.record_l2_miss();
                  Ok(None)
              }
          }
      }

      async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>
      where
          T: Serialize + Send + Sync,
      {
          let serialized = bincode::serialize(value)?;
          let mut conn = self.client.clone();
          let key_owned = self.prefixed_key(key);
          let ttl_secs = ttl.as_secs();

          let result = self.circuit_breaker.call(|| async {
              conn.set_ex::<_, _, ()>(&key_owned, &serialized, ttl_secs)
                  .await
                  .map_err(RedisError::from)?;
              Ok::<_, RedisError>(())
          }).await;

          if result.is_err() {
              self.metrics.record_error();
          }

          Ok(())
      }

      async fn delete(&self, key: &str) -> Result<()> {
          let mut conn = self.client.clone();
          let key_owned = self.prefixed_key(key);

          let result = self.circuit_breaker.call(|| async {
              conn.del::<_, ()>(&key_owned).await.map_err(RedisError::from)?;
              Ok::<_, RedisError>(())
          }).await;

          if result.is_err() {
              self.metrics.record_error();
          }

          Ok(())
      }

      async fn delete_pattern(&self, pattern: &str) -> Result<()> {
          let mut conn = self.client.clone();
          let full_pattern = self.prefixed_key(pattern);

          // Fix #10: Use EVAL with Lua script for atomic SCAN + DEL
          // Re-implementing with a safer SCAN approach for large datasets if needed,
          // but for now KEYS + DEL in Lua is better than wildcard DEL (which doesn't exist).
          let result = self.circuit_breaker.call(|| async {
              let script = redis::Script::new(r#"
                  local keys = redis.call('KEYS', ARGV[1])
                  if #keys > 0 then
                      return redis.call('DEL', unpack(keys))
                  else
                      return 0
                  end
              "#);
              
              script.arg(&full_pattern)
                  .invoke_async::<()>(&mut conn)
                  .await
                  .map_err(RedisError::from)?;
              
              Ok::<_, RedisError>(())
          }).await;

          if result.is_err() {
              self.metrics.record_error();
          }

          Ok(())
      }

      async fn exists(&self, key: &str) -> Result<bool> {
          let mut conn = self.client.clone();
          let key_owned = self.prefixed_key(key);

          let result = self.circuit_breaker.call(|| async {
              let exists: bool = conn.exists(&key_owned).await.map_err(RedisError::from)?;
              Ok::<_, RedisError>(exists)
          }).await;

          match result {
              Ok(exists) => Ok(exists),
              Err(_) => {
                  self.metrics.record_error();
                  Ok(false)
              }
          }
      }

      async fn clear(&self) -> Result<()> {
          // Fix #69: clear() should only clear our namespace, not the whole DB
          self.delete_pattern("*").await
      }

      fn name(&self) -> &'static str {
          "redis"
      }

      fn tier(&self) -> CacheTier {
          CacheTier::L2
      }
  }
