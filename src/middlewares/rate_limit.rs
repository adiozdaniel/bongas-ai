use axum::http::StatusCode;
use redis::AsyncCommands;
use std::sync::Arc;
use tracing::error;

pub struct RateLimiter {
    redis: Arc<redis::Client>,
    max_requests: u64,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(redis: Arc<redis::Client>, max_requests: u64, window_seconds: u64) -> Self {
        Self {
            redis,
            max_requests,
            window_seconds,
        }
    }

    /// Get current rate limit status for an IP
    pub async fn get_status(&self, ip: &str) -> Result<RateLimitStatus, StatusCode> {
        let key = format!("rate_limit:{}", ip);
        let mut conn = self.redis.get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to get Redis connection for rate limit status");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let count: u64 = conn.get(&key).await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to get rate limit count");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let ttl: i64 = conn.ttl(&key).await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to get rate limit TTL");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        Ok(RateLimitStatus {
            limit: self.max_requests,
            window_seconds: self.window_seconds,
            reset_in_seconds: ttl.max(0) as u64,
            is_limited: count > self.max_requests,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub limit: u64,
    pub window_seconds: u64,
    pub reset_in_seconds: u64,
    pub is_limited: bool,
}