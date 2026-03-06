//! L2 Redis cache with circuit breaker protection.

use crate::cache::{CacheStrategy, CacheTier};
use crate::cache::CacheMetrics;
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
use crate::config::RedisConfig;

/// L2 Redis cache with circuit breaker.
pub struct RedisCache {
    client: ConnectionManager,
    circuit_breaker: Arc<CircuitBreaker>,
    metrics: Arc<CacheMetrics>,
    prefix: String,
    config: RedisConfig,
}

impl RedisCache {
    pub async fn new(
        config: RedisConfig,
        metrics: Arc<CacheMetrics>,
    ) -> Result<Self> {
        let client = Client::open(config.url.as_str())
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
            config,
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
        let timeout_dur = Duration::from_secs(self.config.request_timeout);

        let result = self.circuit_breaker.call(|| async {
            let value: Option<Vec<u8>> = tokio::time::timeout(
                timeout_dur, 
                conn.get(&key_owned)
            ).await.map_err(|_| RedisError::Timeout(timeout_dur))??;
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
        let timeout_dur = Duration::from_secs(self.config.request_timeout);

        let result = self.circuit_breaker.call(|| async {
            tokio::time::timeout(
                timeout_dur,
                conn.set_ex::<_, _, ()>(&key_owned, &serialized, ttl_secs)
            ).await.map_err(|_| RedisError::Timeout(timeout_dur))??;
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

        // Fix #10, C1, M2, N1: Use an iterative SCAN approach instead of KEYS
        // This avoids blocking Redis for O(N) operations and prevents Lua stack limits.
        let result = self.circuit_breaker.call(|| async {
            let script = redis::Script::new(r#"
                local cursor = "0"
                local count = 0
                repeat
                    local res = redis.call("SCAN", cursor, "MATCH", ARGV[1], "COUNT", 100)
                    cursor = res[1]
                    local keys = res[2]
                    for i, k in ipairs(keys) do
                        redis.call("DEL", k)
                        count = count + 1
                    end
                until cursor == "0"
                return count
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
