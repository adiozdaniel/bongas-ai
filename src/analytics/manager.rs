//! The central Analytics Manager.
//! Orchestrates various specialized metric modules into a single registry.
//!
//! This module serves as the composition root for all application telemetry.
//! It aggregates individual component metrics collectors and provides a unified
//! Prometheus registry for consistent metric exposition across the entire system.

use std::sync::Arc;
use prometheus::{Encoder, Registry, TextEncoder};
use crate::analytics::{
    clickhouse::ClickHouseMetrics,
    kafka::KafkaMetrics,
    postgres::PostgresMetrics,
    redis::RedisMetrics,
    cache_warming::CacheWarmingMetrics,
    staging::StagingMetrics,
    pipeline::PipelineMetrics,
    middleware::MiddlewareMetrics,
    security::SecurityMetrics,
    experiment::ExperimentMetrics,
    model::ModelMetrics,
    scenario::ScenarioMetrics,
};

/// Centralized metrics orchestrator for the entire application.
///
/// Maintains a single Prometheus registry and delegates component-specific
/// metric registration to specialized collector modules. Provides unified
/// access to all application telemetry for metrics endpoint exposure.
pub struct AnalyticsManager {
    /// The root Prometheus registry containing all application metrics.
    registry: Registry,
    /// Kafka consumer and streaming metrics.
    pub kafka: KafkaMetrics,
    /// ClickHouse database query metrics.
    pub clickhouse: ClickHouseMetrics,
    /// PostgreSQL relational database metrics.
    pub postgres: PostgresMetrics,
    /// Redis cache operation metrics.
    pub redis: RedisMetrics,
    /// Cache warming effectiveness metrics.
    pub cache_warming: CacheWarmingMetrics,
    /// Staging environment and deployment metrics.
    pub staging: StagingMetrics,
    /// Data processing pipeline metrics.
    pub pipeline: PipelineMetrics,
    /// HTTP middleware performance metrics.
    pub middleware: MiddlewareMetrics,
    /// Security and authentication metrics.
    pub security: SecurityMetrics,
    /// Experimentation and bandit algorithm metrics.
    pub experiment: ExperimentMetrics,
    /// ML model inference and performance metrics.
    pub model: ModelMetrics,
    /// Scenario execution and selection metrics.
    pub scenario: ScenarioMetrics,
}

impl AnalyticsManager {
    /// Initializes a new analytics manager with all component metrics registered.
    ///
    /// Creates an empty registry and sequentially registers each specialized
    /// metrics collector. Registration order is deterministic but does not affect
    /// runtime behavior.
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Manager instance or first registration error
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Pass registry by reference to sub-modules
        let kafka = KafkaMetrics::new(&registry)?;
        let clickhouse = ClickHouseMetrics::new(&registry)?;
        let postgres = PostgresMetrics::new(&registry)?;
        let redis = RedisMetrics::new(&registry)?;
        let cache_warming = CacheWarmingMetrics::new(&registry)?;
        let staging = StagingMetrics::new(&registry)?;
        let pipeline = PipelineMetrics::new(&registry)?;
        let middleware = MiddlewareMetrics::new(&registry)?;
        let security = SecurityMetrics::new(&registry)?;
        let experiment = ExperimentMetrics::new(&registry)?;
        let model = ModelMetrics::new(&registry)?;
        let scenario = ScenarioMetrics::new(&registry)?;

        Ok(Self {
            registry,
            kafka,
            clickhouse,
            postgres,
            redis,
            cache_warming,
            staging,
            pipeline,
            middleware,
            security,
            experiment,
            model,
            scenario,
        })
    }

    /// Gathers all registered metrics into Prometheus text format.
    ///
    /// Collects all metric families from the registry, encodes them using the
    /// Prometheus text exposition format, and returns the result as a string.
    /// Suitable for exposing a `/metrics` HTTP endpoint.
    ///
    /// # Returns
    /// * `String` - Prometheus-formatted metrics or empty string on encoding failure
    pub fn gather(&self) -> String {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let _ = encoder.encode(&metric_families, &mut buffer);
        String::from_utf8(buffer).unwrap_or_default()
    }
}

lazy_static::lazy_static! {
    /// Global singleton instance for application-wide telemetry collection.
    ///
    /// Provides thread-safe shared access to the analytics manager from any
    /// component in the system. Initializes once at first access and panics
    /// if metric registration fails.
    pub static ref ANALYTICS: Arc<AnalyticsManager> = Arc::new(
        AnalyticsManager::new().expect("Failed to initialize analytics registry")
    );
}
