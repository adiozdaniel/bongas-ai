//! Analytics module for business and application metrics.
//!
//! Provides metrics collection for business KPIs, user behavior tracking,
//! and application performance monitoring. This is separate from resilience
//! metrics which are handled by the `resilience` module.
//!
//! # Business Analytics Features
//! - **User Behavior Tracking**: Page views, feature usage, conversion rates
//! - **Business KPIs**: Active users, retention, revenue metrics
//! - **Performance Monitoring**: API response times, throughput by endpoint
//! - **Resource Usage**: Memory consumption, CPU usage, disk I/O
//!
//! # Design Patterns
//! - **Strategy**: `AnalyticsExporter` trait for pluggable export formats
//! - **Builder**: `AnalyticsConfig::builder()` for configuration
//! - **Observer**: Event-driven telemetry for business events
//! - **Composite**: `AnalyticsManager` aggregates all business metrics

pub mod config;
pub mod exporter;
pub mod manager;
pub mod types;

// ─── Configuration ──────────────────────────────────────────────────────────

pub use config::AnalyticsConfig;
pub use config::AnalyticsConfigBuilder;
pub use config::AnalyticsConfigError;

// ─── Exporters ──────────────────────────────────────────────────────────────

pub use exporter::AnalyticsExporter;
pub use exporter::JsonExporter;
pub use exporter::PrometheusExporter;

// ─── Manager ────────────────────────────────────────────────────────────────

pub use manager::AnalyticsManager;

// ─── Types ──────────────────────────────────────────────────────────────────

pub use types::BusinessMetrics;
pub use types::UserBehavior;
pub use types::PerformanceMetrics;
pub use types::ResourceMetrics;