//! Configuration types for the Composite Configuration Pattern.
//!
//! Contains all typed configuration structs for the three main modules:
//! circuit breaker, error handling, and analytics, plus supporting types
//! for server, database, and other service configurations.

pub mod app;
pub mod circuit_breaker;
pub mod error;
pub mod analytics;
pub mod server;
pub mod database;
pub mod redis;
pub mod clickhouse;
pub mod kafka;
pub mod security;
pub mod ml;
pub mod pipeline;
pub mod ingestion;
pub mod observability;
pub mod resilience;
pub mod experiments;
pub mod hive_mind;

// Re-export commonly used types
pub use app::AppConfig;
pub use circuit_breaker::{CircuitBreakerConfig, SlidingWindowType};
pub use error::{ErrorConfig, BackoffStrategy};
pub use analytics::{AnalyticsConfig, ExportFormat};
pub use server::ServerConfig;
pub use database::DatabaseConfig;
pub use redis::RedisConfig;
pub use clickhouse::ClickHouseConfig;
pub use kafka::KafkaConfig;
pub use security::SecurityConfig;
pub use ml::{MlConfig, ExposureSourceAdaptor};
pub use pipeline::PipelineConfig;
pub use ingestion::{IngestionConfig, ApiSourceConfig, ClickHouseSourceConfig};
pub use observability::ObservabilityConfig;
pub use resilience::ResilienceConfig;
pub use experiments::ExperimentsConfig;
pub use hive_mind::HiveMindConfig;
