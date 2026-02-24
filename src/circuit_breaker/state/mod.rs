//! Atomic circuit breaker state machine with lock-free transitions.
//!
//! Uses atomic operations for state storage, eliminating the need for
//! mutex locks during state checks.

pub mod models;
pub mod service;

pub use self::models::TransitionResult;
pub use self::service::CircuitBreakerState;
