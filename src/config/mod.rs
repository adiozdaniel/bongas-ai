//! Configuration module for Netflix-grade Composite Configuration Pattern.
//!
//! Provides layered configuration loading with precedence:
//! 1. TOML defaults (config/default.toml)
//! 2. Environment variables (.env)
//! 3. Spring Cloud Config (optional)
//!
//! All configuration is immutable at runtime for maximum throughput.

pub mod http;
pub mod loader;
pub mod sources;
pub mod types;
pub mod validation;

// Re-export main types
pub use loader::ConfigLoader;
pub use types::AppConfig;

// Re-export HTTP config types
pub use http::{CompressionConfig, CorsConfig};

// Re-export config types for convenience
pub use types::{
    AnalyticsConfig, CircuitBreakerConfig, ClickHouseConfig, DatabaseConfig,
    ErrorConfig, IngestionConfig, ApiSourceConfig, ClickHouseSourceConfig,
    MlConfig, PipelineConfig, RedisConfig, SecurityConfig, ServerConfig,
};

// Re-export source types
pub use sources::{ConfigError, ConfigResult, ConfigSource};
