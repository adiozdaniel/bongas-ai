//! Circuit breaker configuration with Builder pattern and full validation.
//!
//! Provides ergonomic, validated configuration for circuit breakers with
//! Netflix Hystrix-style parameters.

pub mod builder;
pub mod error;
pub mod models;

pub use self::builder::CircuitBreakerConfigBuilder;
pub use self::error::ConfigValidationError;
pub use self::models::CircuitBreakerConfig;
