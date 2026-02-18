//! Phase 4: "Nuclear" Edge Security (Multi-Tier Limiter)
//!
//! Protects the engine from scrapers and DDoS via:
//! 1. L1 Local Limiter: DashMap-based sub-100ns lookups.
//! 2. L2 Global Promotion: Redis-based bans for repeat offenders.
//! 3. Shadow Ban: Misleading 200 OK responses with empty feeds.

use axum::http::StatusCode;
use std::sync::Arc;
use std::time::{Instant, Duration};
use dashmap::DashMap;
use tracing::{warn, error};
use serde::{Serialize, Deserialize};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId, CircuitBreakerRegistry};
use crate::error::ErrorClassifier;

const L1_THRESHOLD: u64 = 10; // 10 req/s
const L1_PENALTY_DURATION: Duration = Duration::from_secs(120); // 2 minutes
const L2_PROMOTION_THRESHOLD: u32 = 3; // 3 local blocks = L2 promotion

pub struct RateLimiter {
    l1: DashMap<String, L1State>,
    redis_breaker: Arc<CircuitBreaker>,
    redis_client: Arc<redis::Client>,
    max_requests: u64,
    window_seconds: u64,
}

struct L1State {
    count: u64,
    window_start: Instant,
    blocked_until: Option<Instant>,
    local_block_count: u32,
}

#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed,
    ShadowBan,
    RateLimited(RateLimitStatus),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub limit: u64,
    pub window_seconds: u64,
    pub reset_in_seconds: u64,
    pub is_limited: bool,
}

impl RateLimiter {
    pub fn new(
        redis: Arc<redis::Client>,
        registry: Arc<CircuitBreakerRegistry>,
        max_requests: u64,
        window_seconds: u64,
    ) -> Arc<Self> {
        let breaker = registry.get_or_create(
            CircuitBreakerId::new("redis_rate_limit"),
            CircuitBreakerConfig::default(),
        );

        let limiter = Arc::new(Self {
            l1: DashMap::new(),
            redis_breaker: breaker,
            redis_client: redis,
            max_requests,
            window_seconds,
        });

        // Start janitor task
        let limiter_clone = limiter.clone();
        tokio::spawn(async move {
            limiter_clone.run_janitor().await;
        });

        limiter
    }

    /// Run background cleanup of L1 state.
    async fn run_janitor(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(600)); // Every 10 minutes
        loop {
            interval.tick().await;
            self.evict_expired();
        }
    }

    /// Evict entries that haven't been active for over an hour.
    fn evict_expired(&self) {
        let now = Instant::now();
        let expiry = Duration::from_secs(3600);
        
        let before_count = self.l1.len();
        self.l1.retain(|_, state| {
            now.duration_since(state.window_start) < expiry
        });
        
        let evicted = before_count - self.l1.len();
        if evicted > 0 {
            tracing::debug!(evicted, remaining = self.l1.len(), "Rate limiter L1 janitor completed");
        }
    }

    /// Check rate limit status for an IP.
    pub async fn check(&self, ip: &str) -> RateLimitResult {
        let now = Instant::now();

        // 1. L1 Local Check (Sub-100ns path)
        if let Some(mut entry) = self.l1.get_mut(ip) {
            let state = entry.value_mut();

            // Check if currently blocked locally
            if let Some(until) = state.blocked_until {
                if now < until {
                    return RateLimitResult::ShadowBan;
                } else {
                    state.blocked_until = None;
                }
            }

            // Simple 1s window for L1
            if now.duration_since(state.window_start) >= Duration::from_secs(1) {
                state.count = 1;
                state.window_start = now;
            } else {
                state.count += 1;
            }

            if state.count > L1_THRESHOLD {
                state.blocked_until = Some(now + L1_PENALTY_DURATION);
                state.local_block_count += 1;
                let local_blocks = state.local_block_count;

                warn!(ip = %ip, local_blocks, "L1 Rate Limit Tripped: Local block active");

                if local_blocks >= L2_PROMOTION_THRESHOLD {
                    self.promote_to_l2(ip, local_blocks).await;
                }

                return RateLimitResult::ShadowBan;
            }
        } else {
            self.l1.insert(ip.to_string(), L1State {
                count: 1,
                window_start: now,
                blocked_until: None,
                local_block_count: 0,
            });
        }

        // 2. L2 Global Check (Redis)
        match self.check_l2(ip).await {
            Ok(status) => {
                if status.is_limited {
                    // Global bans are always shadow banned
                    RateLimitResult::ShadowBan
                } else {
                    RateLimitResult::Allowed
                }
            }
            Err(_) => RateLimitResult::Allowed, // Fallback to allow if Redis is down
        }
    }

    async fn check_l2(&self, ip: &str) -> Result<RateLimitStatus, StatusCode> {
        let key = format!("global_ban:{}", ip);
        let redis_client = self.redis_client.clone();
        let key_clone = key.clone();

        let result = self.redis_breaker.call(|| async move {
            let mut conn = redis_client
                .get_multiplexed_async_connection()
                .await
                .map_err(RateLimitError::Connection)?;

            let is_banned: bool = redis::AsyncCommands::exists(&mut conn, &key_clone)
                .await
                .unwrap_or(false);

            if is_banned {
                let ttl: i64 = redis::AsyncCommands::ttl(&mut conn, &key_clone)
                    .await
                    .unwrap_or(0);
                
                Ok::<RateLimitStatus, RateLimitError>(RateLimitStatus {
                    limit: 0,
                    window_seconds: 0,
                    reset_in_seconds: ttl.max(0) as u64,
                    is_limited: true,
                })
            } else {
                Ok::<RateLimitStatus, RateLimitError>(RateLimitStatus {
                    limit: self.max_requests,
                    window_seconds: self.window_seconds,
                    reset_in_seconds: 0,
                    is_limited: false,
                })
            }
        }).await;

        match result {
            Ok(status) => Ok(status),
            Err(e) => {
                error!(error = ?e, ip = %ip, "L2 Rate limit check failed");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    async fn promote_to_l2(&self, ip: &str, local_blocks: u32) {
        let key = format!("global_ban:{}", ip);
        let strike_key = format!("strikes:{}", ip);
        let redis_client = self.redis_client.clone();
        
        let result = self.redis_breaker.call(|| async move {
            let mut conn = redis_client
                .get_multiplexed_async_connection()
                .await
                .map_err(RateLimitError::Connection)?;

            let strikes: u32 = redis::AsyncCommands::incr(&mut conn, &strike_key, 1)
                .await
                .unwrap_or(1);

            let ban_duration = match strikes {
                1 => Duration::from_secs(24 * 3600), // 24 Hours
                _ => Duration::from_secs(7 * 24 * 3600), // 7 Days
            };

            let _: () = redis::AsyncCommands::set_ex(&mut conn, &key, "1", ban_duration.as_secs())
                .await
                .unwrap_or(());

            Ok::<(), RateLimitError>(())
        }).await;

        match result {
            Ok(_) => warn!(ip = %ip, local_blocks, "Promoted to L2: Nuclear Ban active"),
            Err(e) => error!(error = ?e, ip = %ip, "Failed to promote IP to L2"),
        }
    }
}

/// Rate limit error for circuit breaker integration.
#[derive(Debug)]
enum RateLimitError {
    Connection(redis::RedisError),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "Redis connection error: {}", e),
        }
    }
}

impl std::error::Error for RateLimitError {}

impl ErrorClassifier for RateLimitError {
    fn classify(&self) -> crate::error::ErrorClassification {
        use crate::error::ErrorClassification;
        match self {
            Self::Connection(_) => ErrorClassification::Transient,
        }
    }
}
