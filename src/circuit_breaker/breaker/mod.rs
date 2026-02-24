//! Production-grade circuit breaker implementation.
//!
//! This module houses the core orchestrator and public API for the circuit breaker.
//! It coordinates state transitions, metric collection, and event notification.

pub mod error;
pub mod handlers;
pub mod internal;
pub mod models;
pub mod service;

pub use self::error::CircuitBreakerError;
pub use self::models::CircuitBreakerHealth;
pub use self::service::CircuitBreaker;
