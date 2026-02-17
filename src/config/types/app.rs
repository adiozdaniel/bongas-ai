//! Root application configuration for the Composite Configuration Pattern.
//!
//! Contains all configuration for the three main modules: circuit breaker,
//! error handling, and analytics. Provides immutable configuration for
//! Netflix-grade resilience patterns.

use super::{

    CircuitBreakerConfig, ErrorConfig, AnalyticsConfig,

    ServerConfig, DatabaseConfig, RedisConfig, ClickHouseConfig,

    IngestionConfig, SecurityConfig, MlConfig, PipelineConfig,

    ObservabilityConfig, ResilienceConfig, ExperimentsConfig,

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

    pub ingestion: IngestionConfig,

    pub security: SecurityConfig,

    pub ml: MlConfig,

    pub pipeline: PipelineConfig,

    pub circuit_breaker: CircuitBreakerConfig,

    pub error: ErrorConfig,

    pub analytics: AnalyticsConfig,

    pub observability: ObservabilityConfig,

    pub resilience: ResilienceConfig,

    pub experiments: ExperimentsConfig,

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

        circuit_breaker: CircuitBreakerConfig,

        error: ErrorConfig,

        analytics: AnalyticsConfig,

        observability: ObservabilityConfig,

        resilience: ResilienceConfig,

        experiments: ExperimentsConfig,

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

            circuit_breaker,

            error,

            analytics,

            observability,

            resilience,

            experiments,

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

        if self.ingestion.kafka.enabled {

            services.push("ingestion:kafka");

        }

        if self.ingestion.api.enabled {

            services.push("ingestion:api");

        }

        if self.ingestion.clickhouse.enabled {

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

        

        services

    }

}
