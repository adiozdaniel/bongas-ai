//! Redis configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for Redis connection settings and caching behavior.

/// Redis configuration.
///
/// Configuration for Redis connection settings including URL, pool size,
/// and caching behavior.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub request_timeout: u64,
    pub max_retries: u32,
    pub retry_backoff: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
            connection_timeout: 5,
            request_timeout: 10,
            max_retries: 3,
            retry_backoff: 100,
        }
    }
}