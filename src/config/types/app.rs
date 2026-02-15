//! Root application configuration for the Composite Configuration Pattern.
//!
//! Contains all configuration for the three main modules: circuit breaker,
//! error handling, and analytics. Provides immutable configuration for
//! Netflix-grade resilience patterns.

use super::{
    CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,
    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
    KafkaConfig, SecurityConfig, MlConfig, PipelineConfig,
};

/// Root application configuration.
///
/// Immutable configuration structure that contains all configuration
/// for the three main modules: circuit breaker, error handling, and analytics.
/// Follows Netflix's Composite Configuration Pattern with namespace isolation.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub clickhouse: ClickHouseConfig,
    pub kafka: KafkaConfig,
    pub security: SecurityConfig,
    pub ml: MlConfig,
    pub pipeline: PipelineConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub error: ErrorConfig,
    pub analytics: AnalyticsConfig,
}

impl AppConfig {
    /// Create a new application configuration.
    pub fn new(
        server: ServerConfig,
        database: DatabaseConfig,
        redis: RedisConfig,
        clickhouse: ClickHouseConfig,
        kafka: KafkaConfig,
        security: SecurityConfig,
        ml: MlConfig,
        pipeline: PipelineConfig,
        circuit_breaker: CircuitBreakerConfig,
        error: ErrorConfig,
        analytics: AnalyticsConfig,
    ) -> Self {
        Self {
            server,
            database,
            redis,
            clickhouse,
            kafka,
            security,
            ml,
            pipeline,
            circuit_breaker,
            error,
            analytics,
        }
    }

    /// Get enabled services for logging.
    pub fn enabled_services(&self) -> Vec<&'static str> {
        let mut services = Vec::new();
        
        if self.database.url.is_some() {
            services.push("database");
        }
        if !self.redis.url.is_empty() {
            services.push("redis");
        }
        if !self.clickhouse.url.is_empty() {
            services.push("clickhouse");
        }
        if !self.kafka.brokers.is_empty() {
            services.push("kafka");
        }
        if self.circuit_breaker.enabled {
            services.push("circuit_breaker");
        }
        if self.analytics.enabled {
            services.push("analytics");
        }
        
        services
    }
}