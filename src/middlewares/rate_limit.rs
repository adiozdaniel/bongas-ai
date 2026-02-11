use axum::{
    extract::{Request, ConnectInfo},
    http::StatusCode,
    middleware::Next,
    response::{Response, IntoResponse},
    body::Body,
};
use redis::AsyncCommands;
use std::sync::Arc;
use std::net::SocketAddr;
use tracing::{info, warn, error};
use serde_json::json;

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

    pub async fn layer(
        &self,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
        req: Request<Body>,
        next: Next,
    ) -> Result<Response<Body>, StatusCode> {
        let ip = addr.ip().to_string();
        let key = format!("rate_limit:{}", ip);

        let mut conn = self.redis.get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to get Redis connection for rate limiting");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Increment request count
        let count: u64 = conn.incr(&key, 1).await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to increment rate limit counter");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Set expiry on first request
        if count == 1 {
            let _: () = conn.expire(&key, self.window_seconds as i64).await
                .map_err(|e| {
                    warn!(error = ?e, ip = %ip, "Failed to set expiry on rate limit key");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }

        // Check if limit exceeded
        if count > self.max_requests {
            warn!(
                ip = %ip,
                count = count,
                limit = self.max_requests,
                "Rate limit exceeded"
            );

            return Ok(Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("X-RateLimit-Limit", self.max_requests.to_string())
                .header("X-RateLimit-Remaining", "0")
                .header("X-RateLimit-Reset", self.window_seconds.to_string())
                .header("Retry-After", self.window_seconds.to_string())
                .header("Content-Type", "application/json")
                .body(Body::from(json!({
                    "success": false,
                    "error": "Rate limit exceeded",
                    "message": format!("Too many requests. Limit: {} requests per {} seconds", self.max_requests, self.window_seconds),
                    "limit": self.max_requests,
                    "remaining": 0,
                    "reset_time": self.window_seconds,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }).to_string()))
                .unwrap());
        }

        info!(
            ip = %ip,
            count = count,
            limit = self.max_requests,
            "Rate limit check passed"
        );

        // Execute request
        let mut response = next.run(req).await;

        // Add rate limit headers to response
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            self.max_requests.to_string().parse().unwrap(),
        );

        response.headers_mut().insert(
            "X-RateLimit-Remaining",
            (self.max_requests - count).to_string().parse().unwrap(),
        );

        response.headers_mut().insert(
            "X-RateLimit-Reset",
            self.window_seconds.to_string().parse().unwrap(),
        );

        Ok(response)
    }

    /// Check if an IP is currently rate limited
    pub async fn is_limited(&self, ip: &str) -> Result<bool, StatusCode> {
        let key = format!("rate_limit:{}", ip);
        let mut conn = self.redis.get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to get Redis connection for rate limit check");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let count: u64 = conn.get(&key).await
            .map_err(|e| {
                error!(error = ?e, ip = %ip, "Failed to get rate limit count");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        Ok(count > self.max_requests)
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
            ip: ip.to_string(),
            current_requests: count,
            limit: self.max_requests,
            remaining: self.max_requests.saturating_sub(count),
            window_seconds: self.window_seconds,
            reset_in_seconds: ttl.max(0) as u64,
            is_limited: count > self.max_requests,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub ip: String,
    pub current_requests: u64,
    pub limit: u64,
    pub remaining: u64,
    pub window_seconds: u64,
    pub reset_in_seconds: u64,
    pub is_limited: bool,
}

impl RateLimitStatus {
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "ip": self.ip,
            "current_requests": self.current_requests,
            "limit": self.limit,
            "remaining": self.remaining,
            "window_seconds": self.window_seconds,
            "reset_in_seconds": self.reset_in_seconds,
            "is_limited": self.is_limited,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })
    }
}