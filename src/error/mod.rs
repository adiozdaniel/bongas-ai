//! Centralized error module for the Composite Resilience Pattern.
//!
//! Single source of truth for all error types in the application. Domain modules
//! import their error types from here. The resilience infrastructure (circuit
//! breaker, retry, bulkhead) depends only on `ErrorClassification` and
//! `ErrorClassifier` — never on domain specifics.
//!
//! # Design Patterns
//! - **Strategy**: `ErrorClassifier` trait lets each domain define classification.
//! - **Open/Closed**: New domains implement `ErrorClassifier` without modifying
//!   resilience infrastructure.
//! - **Composite**: `AppError` aggregates all domain errors into a single type.

pub mod types;

// Classification infrastructure
pub use types::ErrorClassification;
pub use types::ErrorClassifier;

// Domain errors
pub use types::RedisError;
pub use types::PostgresError;
pub use types::ClickHouseError;
pub use types::KafkaError;
pub use types::CacheError;
pub use types::PipelineError;
pub use types::ModelError;
pub use types::ScenarioError;
pub use types::SecurityError;
pub use types::ExperimentError;
pub use types::MiddlewareError;
pub use types::MetricsError;

// Application-level composite
pub use types::AppError;
pub use types::AppResult;
