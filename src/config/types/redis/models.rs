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
    /// Additional Redis nodes for cluster mode (comma-separated or array)
    pub cluster_nodes: Vec<String>,
    pub pool_size: u32,
    pub connection_timeout: u64,
    pub request_timeout: u64,
    pub max_retries: u32,
    pub retry_backoff: u64,
    /// Enable Redis Cluster mode
    pub cluster_mode: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            cluster_nodes: vec![],
            pool_size: 50, // Increased for concurrent fan-out
            connection_timeout: 5,
            request_timeout: 10,
            max_retries: 3,
            retry_backoff: 100,
            cluster_mode: false,
        }
    }
}

impl RedisConfig {
    /// Get production-grade defaults (5x dev capacity)
    pub fn production() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            cluster_nodes: vec![],
            pool_size: 50, // 5x increase for production
            connection_timeout: 5,
            request_timeout: 10,
            max_retries: 3,
            retry_backoff: 100,
            cluster_mode: false,
        }
    }
}