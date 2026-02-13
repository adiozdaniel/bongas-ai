//! Circuit breaker module for the Composite Resilience Pattern.
//!
//! Provides a generic, production-grade circuit breaker that protects any
//! async operation from cascading failures. Composes rolling window metrics,
//! an explicit state machine, configurable thresholds, observer-based
//! telemetry, and optional bulkhead concurrency limiting.
//!
//! # Design Patterns
//! - **State Machine**: Explicit Closed → Open → HalfOpen transitions.
//! - **Strategy**: `ErrorClassifier` determines breaker behavior per error.
//! - **Builder**: `CircuitBreakerConfig::builder()` for ergonomic config.
//! - **Observer**: Events emitted via `ResilienceObserver` trait.
//! - **Bulkhead**: Optional semaphore-based concurrency isolation.

pub mod breaker;
pub mod config;
pub mod error;
pub mod observer;
pub mod rolling_window;
pub mod state;

pub use breaker::CircuitBreaker;
pub use breaker::CircuitBreakerError;
pub use config::CircuitBreakerConfig;
pub use config::CircuitBreakerConfigBuilder;
pub use rolling_window::WindowSnapshot;
pub use state::StateMachine;
pub use state::TransitionResult;

// Re-export error types
pub use error::ErrorClassification;
pub use error::ErrorClassifier;

// Re-export observer types
pub use observer::CircuitBreakerEvent;
pub use observer::CircuitBreakerId;
pub use observer::CircuitState;
pub use observer::CompositeObserver;
pub use observer::NoOpObserver;
pub use observer::ResilienceObserver;
pub use observer::TracingObserver;
