//! BONGAS-AI library providing Netflix-grade resilience patterns and configuration management.
//!
//! This crate provides the core functionality for the BONGAS-AI service including:
//! - **Configuration Management**: Composite Configuration Pattern with layered loading
//! - **Circuit Breaker**: Netflix Hystrix-inspired circuit breaker with metrics collection
//! - **Error Handling**: Comprehensive error classification and retry strategies
//! - **Analytics**: High-throughput metrics collection and export
//! - **Telemetry**: Observability and distributed tracing
//! - **Security**: License validation and hardware binding
//! - **Caching**: Multi-layer caching with Redis and PostgreSQL
//! - **ML**: Machine learning model management and Candle runtime
//! - **Pipeline**: Recommendation pipeline with dynamic scenarios
//! - **Kafka**: Event streaming with circuit breaker protection
//! - **Database**: PostgreSQL and ClickHouse integration
//! - **API**: RESTful API with middleware stack
//! - **Scenarios**: Dynamic scenario management
//! - **Engine**: Core recommendation engine
//! - **Middlewares**: HTTP middleware stack

// Core modules
pub mod config;
pub mod error;
pub mod telemetry;
pub mod security;

// Resilience modules
pub mod circuit_breaker;
pub mod resilience;
pub mod analytics;

// Data modules
pub mod cache;
pub mod db;
pub mod ingestion;
pub mod ml;
pub mod search;

// Business logic modules
pub mod engine;
pub mod pipeline;
pub mod experiments;
pub mod notification;

// API and infrastructure
pub mod api;
pub mod middlewares;

// Re-export key types for convenience
pub use config::{ConfigLoader, AppConfig};
pub use config::{
    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
    IngestionConfig, SecurityConfig, MlConfig,
    PipelineConfig, AnalyticsConfig, ExperimentsConfig,
};

// Resilience types
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use error::{AppError, AppResult, ErrorClassification, ErrorClassifier};
pub use resilience::{ResilienceMetricsCollector, MetricsRegistry};

// Telemetry
pub use telemetry::{init as initialize_telemetry, TelemetryConfig};
