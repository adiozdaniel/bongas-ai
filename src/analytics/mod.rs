//! Analytics module exports.
//!
//! Re-exports public types and the global analytics manager for convenient
//! consumption by other modules. Provides a single import point for all
//! monitoring and telemetry functionality.

pub mod clickhouse;
pub mod manager;

/// Re-export ClickHouse client for database operations.
pub use self::clickhouse::ClickHouseClient;

/// Re-export analytics components for metrics collection and monitoring.
pub use self::manager::{
    AnalyticsManager,
    AnalyticsMetricsSummary,
    ANALYTICS_MANAGER,
};
