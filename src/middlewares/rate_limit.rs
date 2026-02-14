  //! Netflix-grade rate limiting with circuit breaker protection.
  //!
  //! Integrates with CircuitBreakerRegistry for Redis resilience.

  use axum::http::StatusCode;
  use std::sync::Arc;
  use tracing::error;

  use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId, CircuitBreakerRegistry};
  use crate::error::ErrorClassifier;

  pub struct RateLimiter {
      redis_breaker: Arc<CircuitBreaker>,
      redis_client: Arc<redis::Client>,
      max_requests: u64,
      window_seconds: u64,
  }

  impl RateLimiter {
      /// Create a new rate limiter with circuit breaker protection.
      pub fn new(
          redis: Arc<redis::Client>,
          registry: Arc<CircuitBreakerRegistry>,
          max_requests: u64,
          window_seconds: u64,
      ) -> Self {
          // Get or create circuit breaker for Redis
          let breaker = registry.get_or_create(
              CircuitBreakerId::new("redis_rate_limit"),
              CircuitBreakerConfig::default(),
          );

          Self {
              redis_breaker: breaker,
              redis_client: redis,
              max_requests,
              window_seconds,
          }
      }

      /// Get current rate limit status for an IP with circuit breaker protection.
      pub async fn get_status(&self, ip: &str) -> Result<RateLimitStatus, StatusCode> {
          let key = format!("rate_limit:{}", ip);
          let redis_client = self.redis_client.clone();
          let key_clone = key.clone();

          // Execute through circuit breaker
          let result = self
              .redis_breaker
              .call(|| async move {
                  let mut conn = redis_client
                      .get_multiplexed_async_connection()
                      .await
                      .map_err(RateLimitError::Connection)?;

                  let count: u64 = redis::AsyncCommands::get(&mut conn, &key_clone)
                      .await
                      .unwrap_or(0);

                  let ttl: i64 = redis::AsyncCommands::ttl(&mut conn, &key_clone)
                      .await
                      .unwrap_or(-1);

                  Ok::<(u64, i64), RateLimitError>((count, ttl))
              })
              .await;

          match result {
              Ok((count, ttl)) => Ok(RateLimitStatus {
                  limit: self.max_requests,
                  window_seconds: self.window_seconds,
                  reset_in_seconds: ttl.max(0) as u64,
                  is_limited: count >= self.max_requests,
              }),
              Err(e) => {
                  error!(error = ?e, ip = %ip, "Rate limit check failed");
                  Err(StatusCode::INTERNAL_SERVER_ERROR)
              }
          }
      }
  }

  #[derive(Debug, Clone)]
  pub struct RateLimitStatus {
      pub limit: u64,
      pub window_seconds: u64,
      pub reset_in_seconds: u64,
      pub is_limited: bool,
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

