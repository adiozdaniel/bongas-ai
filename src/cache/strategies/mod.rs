//! Cache strategy implementations.

pub mod lru;
pub mod redis;
pub mod noop;

pub use lru::LruCache;
pub use redis::RedisCache;
pub use noop::NoOpCache;

use anyhow::Result;
use async_trait::async_trait;

use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::cache::traits::{CacheStrategy, CacheTier};

pub enum CacheLayer {
    Lru(LruCache),
    Redis(RedisCache),
}

#[async_trait]
impl CacheStrategy for CacheLayer {
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        match self {
            CacheLayer::Lru(cache) => cache.get(key).await,
            CacheLayer::Redis(cache) => cache.get(key).await,
        }
    }

    async fn set<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        match self {
            CacheLayer::Lru(cache) => cache.set(key, value, ttl).await,
            CacheLayer::Redis(cache) => cache.set(key, value, ttl).await,
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        match self {
            CacheLayer::Lru(cache) => cache.delete(key).await,
            CacheLayer::Redis(cache) => cache.delete(key).await,
        }
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        match self {
            CacheLayer::Lru(cache) => cache.delete_pattern(pattern).await,
            CacheLayer::Redis(cache) => cache.delete_pattern(pattern).await,
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        match self {
            CacheLayer::Lru(cache) => cache.exists(key).await,
            CacheLayer::Redis(cache) => cache.exists(key).await,
        }
    }

    async fn clear(&self) -> Result<()> {
        match self {
            CacheLayer::Lru(cache) => cache.clear().await,
            CacheLayer::Redis(cache) => cache.clear().await,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            CacheLayer::Lru(cache) => cache.name(),
            CacheLayer::Redis(cache) => cache.name(),
        }
    }

    fn tier(&self) -> CacheTier {
        match self {
            CacheLayer::Lru(cache) => cache.tier(),
            CacheLayer::Redis(cache) => cache.tier(),
        }
    }
}
