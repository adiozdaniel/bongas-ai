//! Composite cache layer implementation.

use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::cache::{CacheStrategy, CacheTier};
use super::lru::service::LruCache;
use super::redis::service::RedisCache;

/// Represents a composite cache layer that can be either LRU or Redis.
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

    async fn push_to_list(&self, key: &str, value: String, max_len: usize) -> Result<()> {
        match self {
            CacheLayer::Lru(cache) => cache.push_to_list(key, value, max_len).await,
            CacheLayer::Redis(cache) => cache.push_to_list(key, value, max_len).await,
        }
    }

    async fn get_list(&self, key: &str) -> Result<Vec<String>> {
        match self {
            CacheLayer::Lru(cache) => cache.get_list(key).await,
            CacheLayer::Redis(cache) => cache.get_list(key).await,
        }
    }

    async fn set_raw(&self, key: &str, value: String, ttl: Duration) -> Result<()> {
        match self {
            CacheLayer::Lru(cache) => cache.set_raw(key, value, ttl).await,
            CacheLayer::Redis(cache) => cache.set_raw(key, value, ttl).await,
        }
    }

    async fn get_raw(&self, key: &str) -> Result<Option<String>> {
        match self {
            CacheLayer::Lru(cache) => cache.get_raw(key).await,
            CacheLayer::Redis(cache) => cache.get_raw(key).await,
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
