use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    /// Set key with expiration (seconds) and track cache metrics
    pub async fn set_ex(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        // Start cache lookup timer
        let _timer = crate::analytics::ANALYTICS_MANAGER.start_cache_lookup_timer("redis");
        
        let mut conn = self.manager.clone();
        conn.set_ex::<_, _, ()>(key, value, ttl_seconds).await?;
        
        Ok(())
    }

    /// Get key and track cache hit/miss metrics
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        // Start cache lookup timer
        let _timer = crate::analytics::ANALYTICS_MANAGER.start_cache_lookup_timer("redis");
        
        let mut conn = self.manager.clone();
        let value: Option<String> = conn.get(key).await?;
        
        // Track cache hit/miss (separate from timing)
        if value.is_some() {
            crate::analytics::ANALYTICS_MANAGER.record_cache_hit("redis", "key");
        } else {
            crate::analytics::ANALYTICS_MANAGER.record_cache_miss("redis", "key");
        }
        
        Ok(value)
    }

    /// Delete key and track cache eviction metrics
    pub async fn del(&self, key: &str) -> Result<()> {
        // Start cache lookup timer
        let _timer = crate::analytics::ANALYTICS_MANAGER.start_cache_lookup_timer("redis");
        
        let mut conn = self.manager.clone();
        conn.del::<_, ()>(key).await?;
        
        // Track cache eviction
        crate::analytics::ANALYTICS_MANAGER.record_cache_eviction("redis");
        
        Ok(())
    }

    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self { manager })
    }
}
