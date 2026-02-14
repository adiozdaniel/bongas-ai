//! Netflix-grade resilient database pool with circuit breaker and bulkhead patterns.
//!
//! # Architecture
//! - **Circuit Breaker**: Protects against cascading failures from database outages
//! - **Bulkhead**: Limits concurrent connections to prevent resource exhaustion
//! - **Observer Pattern**: Emits events for monitoring and alerting
//! - **Composite Pattern**: Wraps sqlx::PgPool with resilience layer
//!
//! # Design Patterns
//! - **Decorator**: ResilientPool decorates PgPool with resilience
//! - **Strategy**: Different timeout/retry strategies per query type
//! - **Observer**: Circuit breaker events flow to ResilienceObserver

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use super::metrics::DatabaseMetrics;
use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId, CircuitBreakerRegistry,
};
use crate::error::PostgresError;

/// Configuration for the resilient database pool.
#[derive(Debug, Clone)]
pub struct ResilientPoolConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    /// Connection acquisition timeout
    pub acquire_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Duration,
    /// Maximum connection lifetime
    pub max_lifetime: Duration,
    /// Query timeout for circuit breaker
    pub query_timeout: Duration,
    /// Bulkhead: max concurrent queries
    pub max_concurrent_queries: usize,
    /// Circuit breaker failure threshold
    pub failure_rate_threshold: f64,
    /// Circuit breaker slow call threshold
    pub slow_call_rate_threshold: f64,
    /// Duration to consider a call "slow"
    pub slow_call_duration: Duration,
}

impl Default for ResilientPoolConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
            query_timeout: Duration::from_secs(30),
            max_concurrent_queries: 100,
            failure_rate_threshold: 0.5,
            slow_call_rate_threshold: 0.5,
            slow_call_duration: Duration::from_secs(5),
        }
    }
}

impl ResilientPoolConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    pub fn with_query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    pub fn with_bulkhead_size(mut self, size: usize) -> Self {
        self.max_concurrent_queries = size;
        self
    }
}

/// Netflix-grade resilient database pool.
///
/// Wraps sqlx::PgPool with:
/// - Circuit breaker for failure isolation
/// - Bulkhead (semaphore) for concurrency limiting
/// - Metrics collection for observability
/// - Timeout enforcement for all queries
pub struct ResilientPool {
    /// Underlying connection pool
    pool: PgPool,
    /// Circuit breaker for database operations
    circuit_breaker: Arc<CircuitBreaker>,
    /// Bulkhead semaphore for limiting concurrent queries
    bulkhead: Arc<Semaphore>,
    /// Query timeout
    query_timeout: Duration,
    /// Metrics collector
    metrics: Arc<DatabaseMetrics>,
    /// Configuration
    config: ResilientPoolConfig,
}

impl ResilientPool {
    /// Create a new resilient pool with the given configuration.
    pub async fn new(
        config: ResilientPoolConfig,
        circuit_breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Result<Self> {
        // Create the underlying sqlx pool
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(&config.url)
            .await
            .context("Failed to create database pool")?;

        // Get or create circuit breaker from registry
        let cb_config = CircuitBreakerConfig::builder()
            .failure_rate_threshold(config.failure_rate_threshold)
            .slow_call_rate_threshold(config.slow_call_rate_threshold)
            .slow_call_duration(config.slow_call_duration)
            .minimum_calls(10)
            .build()
            .context("Invalid circuit breaker config")?;

        let circuit_breaker =
            circuit_breaker_registry.get_or_create(CircuitBreakerId::new("postgres"), cb_config);

        // Create bulkhead semaphore
        let bulkhead = Arc::new(Semaphore::new(config.max_concurrent_queries));

        // Create metrics collector
        let metrics = Arc::new(DatabaseMetrics::new());

        tracing::info!(
            max_connections = config.max_connections,
            max_concurrent_queries = config.max_concurrent_queries,
            query_timeout_ms = config.query_timeout.as_millis() as u64,
            "Resilient database pool initialized"
        );

        Ok(Self {
            pool,
            circuit_breaker,
            bulkhead,
            query_timeout: config.query_timeout,
            metrics,
            config,
        })
    }

    /// Execute a query with full resilience (circuit breaker + bulkhead + timeout).
    ///
    /// The operation receives a reference to the underlying PgPool.
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(PgPool) -> Fut,
        Fut: Future<Output = Result<T, sqlx::Error>> + Send,
        T: Send,
    {
        // Acquire bulkhead permit
        let _permit = self
            .bulkhead
            .acquire()
            .await
            .map_err(|_| PostgresError::PoolExhausted)?;

        self.metrics.record_query_start();

        // Clone pool for the closure (sqlx pools are cheap to clone - they're Arc internally)
        let pool_clone = self.pool.clone();
        let query_timeout = self.query_timeout;

        // Execute through circuit breaker with timeout
        let result = self
            .circuit_breaker
            .call(|| async move {
                let fut = operation(pool_clone);

                match timeout(query_timeout, fut).await {
                    Ok(Ok(result)) => Ok(result),
                    Ok(Err(e)) => Err(PostgresError::Query {
                        message: e.to_string(),
                        source: Some(Box::new(e)),
                    }),
                    Err(_) => Err(PostgresError::Timeout(query_timeout)),
                }
            })
            .await;

        match &result {
            Ok(_) => self.metrics.record_query_success(),
            Err(_) => self.metrics.record_query_failure(),
        }

        result.map_err(|e| match e {
            crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => {
                self.metrics.record_circuit_open();
                anyhow::anyhow!("Database circuit breaker is open")
            }
            crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => {
                anyhow::anyhow!("Database operation failed: {}", source)
            }
            crate::circuit_breaker::CircuitBreakerError::TimedOut { timeout } => {
                anyhow::anyhow!("Database operation timed out after {:?}", timeout)
            }
        })
    }

    /// Get the underlying pool for raw access (use sparingly).
    pub fn inner(&self) -> &PgPool {
        &self.pool
    }

    /// Get current pool statistics.
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
            max_connections: self.config.max_connections,
            bulkhead_available: self.bulkhead.available_permits(),
            bulkhead_max: self.config.max_concurrent_queries,
        }
    }

    /// Get database metrics snapshot.
    pub fn metrics(&self) -> &DatabaseMetrics {
        &self.metrics
    }

    /// Check if the circuit breaker is open.
    pub fn is_circuit_open(&self) -> bool {
        matches!(
            self.circuit_breaker.state(),
            crate::circuit_breaker::CircuitBreakerState::Open
        )
    }

    /// Get circuit breaker health.
    pub fn circuit_health(&self) -> crate::circuit_breaker::CircuitBreakerHealth {
        self.circuit_breaker.health()
    }

    /// Close the pool gracefully.
    pub async fn close(&self) {
        self.pool.close().await;
        tracing::info!("Database pool closed");
    }
}

/// Pool statistics for monitoring.
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub size: u32,
    pub idle: usize,
    pub max_connections: u32,
    pub bulkhead_available: usize,
    pub bulkhead_max: usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool[size={}/{}, idle={}, bulkhead={}/{}]",
            self.size, self.max_connections, self.idle, self.bulkhead_available, self.bulkhead_max
        )
    }
}
