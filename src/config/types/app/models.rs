//! Root application configuration for the Composite Configuration Pattern.
//!
//! Contains all configuration for the three main modules: circuit breaker,
//! error handling, and analytics. Provides immutable configuration for
//! Netflix-grade resilience patterns.

use crate::config::types::{
    CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,
    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,
    IngestionConfig, SecurityConfig, MlConfig, PipelineConfig,
    ObservabilityConfig, ResilienceConfig, ExperimentsConfig, HiveMindConfig,
};
use crate::cache::CacheConfig;
use crate::resilience::ResilienceMetricsConfig;

/// Root application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub clickhouse: ClickHouseConfig,
    pub ingestion: IngestionConfig,
    pub security: SecurityConfig,
    pub ml: MlConfig,
    pub pipeline: PipelineConfig,
    pub cache: CacheConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub error: ErrorConfig,
    pub analytics: AnalyticsConfig,
    pub observability: ObservabilityConfig,
    pub resilience: ResilienceConfig,
    pub resilience_metrics: ResilienceMetricsConfig,
    pub experiments: ExperimentsConfig,
    pub hive_mind: HiveMindConfig,
}

impl AppConfig {
    /// Create a new application configuration.
    pub fn new(
        server: ServerConfig,
        database: DatabaseConfig,
        redis: RedisConfig,
        clickhouse: ClickHouseConfig,
        ingestion: IngestionConfig,
        security: SecurityConfig,
        ml: MlConfig,
        pipeline: PipelineConfig,
        cache: CacheConfig,
        circuit_breaker: CircuitBreakerConfig,
        error: ErrorConfig,
        analytics: AnalyticsConfig,
        observability: ObservabilityConfig,
        resilience: ResilienceConfig,
        resilience_metrics: ResilienceMetricsConfig,
        experiments: ExperimentsConfig,
        hive_mind: HiveMindConfig,
    ) -> Self {
        Self {
            server,
            database,
            redis,
            clickhouse,
            ingestion,
            security,
            ml,
            pipeline,
            cache,
            circuit_breaker,
            error,
            analytics,
            observability,
            resilience,
            resilience_metrics,
            experiments,
            hive_mind,
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

        if !self.ingestion.kafka.brokers.is_empty() {
            services.push("ingestion:kafka");
        }

        services.push("ingestion:api");

        if !self.clickhouse.url.is_empty() {
            services.push("ingestion:clickhouse");
        }

        if self.circuit_breaker.enabled {

            services.push("circuit_breaker");

        }

        if self.analytics.enabled {

            services.push("analytics");

        }

        if self.experiments.enabled {

            services.push("experiments");

        }

        if self.hive_mind.enabled {

            services.push("hive_mind");

        }

        

        services

    }

}
