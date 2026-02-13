//! Analytics module root.
//!
//! Re-exports all specialized metric collectors and provides the global analytics
//! manager instance. This module serves as the single entry point for all application
//! telemetry, centralizing metric definitions and collection.

pub mod clickhouse;
pub mod kafka;
pub mod manager;
pub mod metrics;
pub mod postgres;
pub mod redis;
pub mod cache_warming;
pub mod staging;
pub mod pipeline;
pub mod middleware;
pub mod security;
pub mod experiment;
pub mod model;
pub mod scenario;

/// Global singleton instance for application-wide metrics collection.
///
/// Provides thread-safe access to all specialized metric collectors. Initialize
/// once at application startup and use throughout the codebase for consistent
/// telemetry recording.
pub use manager::ANALYTICS;

/*
Usage Example:
--------------
use crate::analytics::ANALYTICS;

// Increment Kafka message counter
ANALYTICS.kafka.messages_sent.with_label_values(&["user_events"]).inc();

// Measure ClickHouse query latency
let timer = ANALYTICS.db.query_latency.with_label_values(&["users"]).start_timer();
// ... execute query ...
timer.observe_duration();

// Record cache warming operation
ANALYTICS.cache_warming.warming_attempts.with_label_values(&["popular_items"]).inc();
*/
