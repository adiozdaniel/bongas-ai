//! Production-grade circuit breaker module for the Composite Resilience Pattern.
//!
//! Provides a Netflix Hystrix-inspired circuit breaker that protects any
//! async operation from cascading failures. Features include:
//!
//! - **Lock-free metrics**: Rolling window with atomic counters
//! - **Atomic state machine**: Race-free state transitions
//! - **Slow call detection**: Trip on latency degradation
//! - **Consecutive failure tracking**: Alternative to rate-based tripping
//! - **Central registry**: Manage all breakers from one place
//! - **Health introspection**: Dashboard-ready health endpoints
//! - **Observer pattern**: Pluggable telemetry (tracing, Prometheus, etc.)
//! - **Bulkhead**: Optional semaphore-based concurrency limiting
//!
//! # Design Patterns
//! - **State Machine**: Explicit Closed → Open → HalfOpen transitions
//! - **Strategy**: `ErrorClassifier` determines breaker behavior per error
//! - **Builder**: `CircuitBreakerConfig::builder()` for ergonomic config
//! - **Observer**: Events emitted via `ResilienceObserver` trait
//! - **Registry**: Central management of all circuit breakers
//!
//! # Example
//! ```ignore
//! use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerId};
//!
//! let config = CircuitBreakerConfig::builder()
//!     .failure_rate_threshold(0.5)
//!     .slow_call_rate_threshold(0.5)
//!     .slow_call_duration(Duration::from_secs(2))
//!     .minimum_calls(10)
//!     .build()?;
//!
//! let breaker = CircuitBreaker::new(
//!     CircuitBreakerId::new("database"),
//!     config,
//!     Arc::new(TracingObserver),
//! );
//!
//! let result = breaker.call(|| async {
//!     database.query("SELECT 1").await
//! }).await;
//! ```

pub mod breaker;
pub mod config;
pub mod observer;
pub mod registry;
pub mod rolling_window;
pub mod state;

// ─── Primary Types ─────────────────────────────────────────────────────────

pub use breaker::CircuitBreaker;
pub use breaker::CircuitBreakerError;
pub use breaker::CircuitBreakerHealth;

// ─── Configuration ─────────────────────────────────────────────────────────

pub use config::CircuitBreakerConfig;
pub use config::CircuitBreakerConfigBuilder;
pub use config::ConfigValidationError;

// ─── Registry ──────────────────────────────────────────────────────────────

pub use registry::CircuitBreakerRegistry;
pub use registry::RegistryStateSummary;

// ─── State & Metrics ───────────────────────────────────────────────────────

pub use rolling_window::WindowSnapshot;
pub use state::CircuitBreakerState;
pub use state::TransitionResult;

// ─── Observer Types ────────────────────────────────────────────────────────

pub use observer::CircuitBreakerEvent;
pub use observer::CircuitBreakerId;
pub use observer::CircuitState;
pub use observer::CompositeObserver;
pub use observer::NoOpObserver;
pub use observer::ResilienceObserver;
pub use observer::TracingObserver;
