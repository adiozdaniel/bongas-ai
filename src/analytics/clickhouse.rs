//! ClickHouse-specific instrumentation.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, 
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

pub struct ClickHouseMetrics {
    pub queries_total: IntCounterVec,
    pub query_latency: HistogramVec,
}

impl ClickHouseMetrics {
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
