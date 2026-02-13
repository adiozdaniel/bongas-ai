//! Shared constants for Prometheus metrics to ensure consistency.

pub const NAMESPACE: &str = "bongas";

pub mod labels {
    pub const TOPIC: &str = "topic";
    pub const GROUP_ID: &str = "group_id";
    pub const TABLE: &str = "table";
    pub const QUERY_TYPE: &str = "query_type";
}

/// Latency buckets optimized for microservices (in seconds).
pub const DEFAULT_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];
