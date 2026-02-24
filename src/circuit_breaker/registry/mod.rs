//! Central circuit breaker registry for management and introspection.
//!
//! Provides a singleton-style registry for all circuit breakers in the application.

pub mod models;
pub mod service;

pub use self::models::RegistryStateSummary;
pub use self::service::CircuitBreakerRegistry;
