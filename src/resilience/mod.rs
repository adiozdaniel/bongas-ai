//! Resilience module for the Composite Resilience Pattern.
//!
//! Provides high-throughput metrics collection, aggregation, and export
//! for all circuit breakers in the system. Lock-free where possible,
//! with HDR histograms for accurate latency percentiles.
//!
//! # Netflix Resilience Features
//! - **Error Classification Tracking**: Breakdown by classification type
//! - **Degraded/Partial Failure Metrics**: Tracked separately
//! - **Lock Poison Recovery**: All locks recover gracefully from panics
//! - **Validated Configuration**: Builder pattern with cross-field validation
//!
//! # Design Patterns
//! - **Observer**: `ResilienceMetricsCollector` receives circuit breaker events.
//! - **Strategy**: `MetricsExporter` trait for pluggable export formats.
//! - **Builder**: `ResilienceConfig::builder()` for configuration.
//! - **Composite**: `MetricsRegistry` aggregates all breaker metrics.
//! - **Flyweight**: Shared histogram buckets for memory efficiency.

pub mod collector;
pub mod config;
pub mod exporter;
pub mod histogram;
pub mod registry;
pub mod types;

// ─── Collector ──────────────────────────────────────────────────────────────

pub use collector::ResilienceMetricsCollector;

// ─── Configuration ──────────────────────────────────────────────────────────

pub use config::ResilienceConfig;
pub use config::ResilienceConfigBuilder;
pub use config::ResilienceConfigError;

// ─── Exporters ──────────────────────────────────────────────────────────────

pub use exporter::JsonExporter;
pub use exporter::MetricsExporter;
pub use exporter::PrometheusExporter;

// ─── Histogram ──────────────────────────────────────────────────────────────

pub use histogram::HdrHistogram;

// ─── Registry ───────────────────────────────────────────────────────────────

pub use registry::BreakerMetrics;
pub use registry::MetricsRegistry;

// ─── Types ──────────────────────────────────────────────────────────────────

pub use types::BreakerSnapshot;
pub use types::ClassificationCounters;
pub use types::ClassificationSnapshot;
pub use types::Counter;
pub use types::Gauge;
pub use types::Rate;
pub use types::RegistrySnapshot;
