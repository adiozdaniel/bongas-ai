//! ClickHouse-specific instrumentation.
//!
//! Provides Prometheus metrics for monitoring database query performance.
//! Tracks query volume, type distribution, table access patterns, and
//! execution latency distributions.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for ClickHouse database operations.
///
/// Maintains counters and histograms for:
/// * Query throughput by operation type and target table
/// * Query latency percentiles for performance monitoring
pub struct ClickHouseMetrics {
    /// Total number of database queries, labeled by query type and table.
    pub queries_total: IntCounterVec,
    /// Query execution time distribution, labeled by table.
    pub query_latency: HistogramVec,
}

impl ClickHouseMetrics {
    /// Creates and registers ClickHouse metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            queries_total: register_int_counter_vec_with_registry!(
                opts!("clickhouse_queries_total", "Total database queries"),
                &[labels::QUERY_TYPE, labels::TABLE],
                registry
            )?,
            query_latency: register_histogram_vec_with_registry!(
                format!("{}_clickhouse_query_duration_seconds", NAMESPACE),
                "Query execution time",
                &[labels::TABLE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
