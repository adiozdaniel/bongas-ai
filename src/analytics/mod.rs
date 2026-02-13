//! Analytics module for the Composite Resilience Pattern.
//!
//! Provides high-throughput metrics collection, aggregation, and export
//! for all circuit breakers in the system. Lock-free where possible,
//! with HDR histograms for accurate latency percentiles.
//!
//! # Design Patterns
//! - **Observer**: `ResilienceMetricsCollector` receives circuit breaker events.
//! - **Strategy**: `MetricsExporter` trait for pluggable export formats.
//! - **Builder**: `AnalyticsConfig::builder()` for configuration.
//! - **Composite**: `MetricsRegistry` aggregates all breaker metrics.
//! - **Flyweight**: Shared histogram buckets for memory efficiency.

pub mod collector;
pub mod config;
pub mod exporter;
pub mod histogram;
pub mod registry;
pub mod types;

pub use collector::ResilienceMetricsCollector;
pub use config::AnalyticsConfig;
pub use config::AnalyticsConfigBuilder;
pub use exporter::JsonExporter;
pub use exporter::MetricsExporter;
pub use exporter::PrometheusExporter;
pub use histogram::HdrHistogram;
pub use registry::BreakerMetrics;
pub use registry::MetricsRegistry;
pub use types::BreakerSnapshot;
pub use types::Counter;
pub use types::Gauge;
pub use types::Rate;
pub use types::RegistrySnapshot;
