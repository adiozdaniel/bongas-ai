//! Observer module for the Composite Resilience Pattern.
//!
//! Decouples event emission from event handling. Resilience infrastructure
//! (circuit breaker, retry, bulkhead) emits events through the
//! `ResilienceObserver` trait. Consumers implement the trait to record
//! Prometheus metrics, emit structured logs, or forward to external systems.
//!
//! # Design Patterns
//! - **Observer**: Resilience components notify without knowing who listens.
//! - **Composite**: `CompositeObserver` fans out to multiple observers.
//! - **Null Object**: `NoOpObserver` eliminates `Option` checks.

pub mod event;
pub mod traits;

pub use event::CircuitBreakerEvent;
pub use event::CircuitBreakerId;
pub use event::CircuitState;

pub use traits::CompositeObserver;
pub use traits::NoOpObserver;
pub use traits::ResilienceObserver;
pub use traits::TracingObserver;
