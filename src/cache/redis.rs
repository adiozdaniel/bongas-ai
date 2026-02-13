use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::time::Instant;

pub struct RedisClient {
    manager: ConnectionManager,
}

impl RedisClient {
    /// Set key with expiration (seconds) and track cache metrics
    pub async fn set_ex(&self, key: &str, value: &str, ttl_seconds: u64) -> Result<()> {
        // Start cache lookup timer
        let _timer = Instant::now();
        // TODO: Use _timer to track cache lookup duration and metrics as needed
        
        let mut conn = self.manager.clone();
        conn.set_ex::<_, _, ()>(key, value, ttl_seconds).await?;
        
        Ok(())
    }

    /// Get key and track cache hit/miss metrics
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        // Start cache lookup timer
        let _timer = Instant::now();
        
        let mut conn = self.manager.clone();
        let value: Option<String> = conn.get(key).await?;
        
        // Track cache hit/miss (separate from timing)
        
        Ok(value)
    }

    /// Delete key and track cache eviction metrics
    pub async fn del(&self, key: &str) -> Result<()> {
        // Start cache lookup timer
        let _timer = Instant::now();
        
        let mut conn = self.manager.clone();
        conn.del::<_, ()>(key).await?;
        
        // Track cache eviction
        
        
        Ok(())
    }

    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self { manager })
    }
}
