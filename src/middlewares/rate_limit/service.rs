//! Phase 4: "Nuclear" Edge Security (Multi-Tier Limiter)
//!
//! Protects the engine from scrapers and DDoS via:
//! 1. L1 Local Limiter: DashMap-based sub-100ns lookups.
//! 2. L2 Global Promotion: Redis-based bans for repeat offenders.
//! 3. Shadow Ban: Misleading 200 OK responses with empty feeds.

use axum::http::StatusCode;
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    extract::Extension,
};
use std::sync::Arc;
use std::time::{Instant, Duration};
use dashmap::DashMap;
use tracing::{warn, error, info};
use serde::{Serialize, Deserialize};

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId, CircuitBreakerRegistry};
use crate::error::ErrorClassifier;

const L1_PENALTY_DURATION: Duration = Duration::from_secs(120); // 2 minutes
const L2_PROMOTION_THRESHOLD: u32 = 3; // 3 local blocks = L2 promotion

use redis::aio::ConnectionManager;

pub struct RateLimiter {
    l1: DashMap<String, L1State>,
    redis_breaker: Arc<CircuitBreaker>,
    redis_client: Arc<redis::Client>,
    redis_manager: Arc<tokio::sync::RwLock<Option<ConnectionManager>>>,
    max_requests: u64,
    window_seconds: u64,
    shutdown_rx: Option<tokio::sync::broadcast::Receiver<()>>,
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
        shutdown_rx: Option<tokio::sync::broadcast::Receiver<()>>,
    ) -> Arc<Self> {
        let breaker = registry.get_or_create(
            CircuitBreakerId::new("redis_rate_limit"),
            CircuitBreakerConfig::default(),
        );

        let limiter = Arc::new(Self {
            l1: DashMap::new(),
            redis_breaker: breaker,
            redis_client: redis.clone(),
            redis_manager: Arc::new(tokio::sync::RwLock::new(None)),
            max_requests,
            window_seconds,
            shutdown_rx,
        });

        // Initialize ConnectionManager in background
        let manager_arc = limiter.redis_manager.clone();
        let redis_inner = redis.clone();
        tokio::spawn(async move {
            match ConnectionManager::new((*redis_inner).clone()).await {
                Ok(cm) => {
                    *manager_arc.write().await = Some(cm);
                    info!("Rate limiter Redis ConnectionManager initialized");
                }
                Err(e) => {
                    error!("Failed to initialize rate limiter Redis manager: {}", e);
                }
            }
        });

        // Start janitor task
        let limiter_clone = limiter.clone();
        tokio::spawn(async move {
            limiter_clone.run_janitor().await;
        });

        limiter
    }

    pub async fn layer(
        Extension(state): Extension<Arc<Self>>,
        req: Request<Body>,
        next: Next,
    ) -> Response {
        let ip = req.extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|axum::extract::ConnectInfo(addr)| addr.ip().to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        match state.check(&ip).await {
            RateLimitResult::Allowed => next.run(req).await,
            RateLimitResult::ShadowBan => {
                // Shadow ban: return 200 OK but with misleading empty body
                // For recommendations, this would be an empty feed
                StatusCode::OK.into_response()
            }
            RateLimitResult::RateLimited(_) => {
                StatusCode::TOO_MANY_REQUESTS.into_response()
            }
        }
    }

    async fn get_redis_conn(&self) -> Result<ConnectionManager, RateLimitError> {
        let manager = self.redis_manager.read().await;
        if let Some(ref cm) = *manager {
            Ok(cm.clone())
        } else {
            // Lazy initialization if background task didn't finish yet or failed
            drop(manager);
            let mut manager = self.redis_manager.write().await;
            if let Some(ref cm) = *manager {
                Ok(cm.clone())
            } else {
                let cm = ConnectionManager::new((*self.redis_client).clone()).await
                    .map_err(RateLimitError::Connection)?;
                *manager = Some(cm.clone());
                Ok(cm)
            }
        }
    }

    /// Run background cleanup of L1 state.
    async fn run_janitor(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(600)); // Every 10 minutes
        let mut shutdown = self.shutdown_rx.as_ref().map(|rx| rx.resubscribe());

        loop {
            if let Some(ref mut rx) = shutdown {
                tokio::select! {
                    _ = interval.tick() => self.evict_expired(),
                    _ = rx.recv() => {
                        warn!("Rate limiter janitor shutting down...");
                        break;
                    }
                }
            } else {
                interval.tick().await;
                self.evict_expired();
            }
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

        // 1. L1 Local Check (Sub-100ns path) - Using entry API to fix TOCTOU race
        let mut local_block_active = false;
        let mut local_blocks = 0;

        self.l1.entry(ip.to_string())
            .and_modify(|state| {
                // Check if currently blocked locally
                if let Some(until) = state.blocked_until {
                    if now < until {
                        local_block_active = true;
                        return;
                    } else {
                        state.blocked_until = None;
                    }
                }

                // Dynamic window based on config
                if now.duration_since(state.window_start) >= Duration::from_secs(self.window_seconds) {
                    state.count = 1;
                    state.window_start = now;
                } else {
                    state.count += 1;
                }

                if state.count > self.max_requests {
                    state.blocked_until = Some(now + L1_PENALTY_DURATION);
                    state.local_block_count += 1;
                    local_blocks = state.local_block_count;
                    local_block_active = true;
                }
            })
            .or_insert(L1State {
                count: 1,
                window_start: now,
                blocked_until: None,
                local_block_count: 0,
            });

        if local_block_active {
            if local_blocks >= L2_PROMOTION_THRESHOLD {
                warn!(ip = %ip, local_blocks, "L1 Rate Limit Tripped: Promoting to L2 ban");
                self.promote_to_l2(ip, local_blocks).await;
                return RateLimitResult::ShadowBan;
            } else {
                warn!(ip = %ip, "L1 Rate Limit Tripped: Local penalty active");
                return RateLimitResult::RateLimited(RateLimitStatus {
                    limit: self.max_requests,
                    window_seconds: self.window_seconds,
                    reset_in_seconds: L1_PENALTY_DURATION.as_secs(),
                    is_limited: true,
                });
            }
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
        let key_clone = key.clone();
        let this = self;

        let result = self.redis_breaker.call(|| async move {
            let mut conn = this.get_redis_conn().await?;

            let is_banned: bool = redis::AsyncCommands::exists(&mut conn, &key_clone)
                .await
                .map_err(RateLimitError::Connection)?;

            if is_banned {
                let ttl: i64 = redis::AsyncCommands::ttl(&mut conn, &key_clone)
                    .await
                    .map_err(RateLimitError::Connection)?;
                
                Ok::<RateLimitStatus, RateLimitError>(RateLimitStatus {
                    limit: 0,
                    window_seconds: 0,
                    reset_in_seconds: ttl.max(0) as u64,
                    is_limited: true,
                })
            } else {
                Ok::<RateLimitStatus, RateLimitError>(RateLimitStatus {
                    limit: this.max_requests,
                    window_seconds: this.window_seconds,
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
        let this = self;
        
        let result = self.redis_breaker.call(|| async move {
            let mut conn = this.get_redis_conn().await?;

            let strikes: u32 = redis::AsyncCommands::incr(&mut conn, &strike_key, 1)
                .await
                .map_err(RateLimitError::Connection)?;

            let ban_duration = match strikes {
                1 => Duration::from_secs(24 * 3600), // 24 Hours
                _ => Duration::from_secs(7 * 24 * 3600), // 7 Days
            };

            let _: () = redis::AsyncCommands::set_ex(&mut conn, &key, "1", ban_duration.as_secs())
                .await
                .map_err(RateLimitError::Connection)?;

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
