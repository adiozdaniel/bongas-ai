//! Redis-specific instrumentation for L1 cache operations.
//!
//! Provides Prometheus metrics for monitoring Redis cache performance and operations.
//! Tracks operation throughput, success rates, latency distributions, connection pool
//! status, memory utilization, and key-level operation patterns. Enables observability
//! of the L1 caching layer performance and health.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for Redis cache operations.
///
/// Maintains comprehensive metric vectors for:
/// * Cache operation throughput and reliability (GET, SET, DELETE, etc.)
/// * Operation latency distributions for performance monitoring
/// * Connection pool sizing and utilization
/// * Memory consumption patterns and trends
/// * Key-level operation distribution
pub struct RedisMetrics {
    /// Total Redis operations, labeled by operation type.
    pub redis_operations: IntCounterVec,
    /// Successful Redis operations, labeled by operation type.
    pub redis_successes: IntCounterVec,
    /// Failed Redis operations, labeled by operation type.
    pub redis_failures: IntCounterVec,
    /// Redis operation latency distribution, labeled by operation type.
    pub redis_operation_latency: HistogramVec,

    /// Current Redis connection pool size, labeled by pool type.
    pub redis_connection_pool_size: IntCounterVec,
    /// Redis memory usage distribution in bytes, labeled by memory type.
    pub redis_memory_usage: HistogramVec,
    /// Total Redis key operations, labeled by key operation type.
    pub redis_key_operations: IntCounterVec,
}

impl RedisMetrics {
    /// Creates and registers all Redis metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Redis-specific operations
            redis_operations: register_int_counter_vec_with_registry!(
                opts!("redis_operations_total", "Total Redis operations"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            redis_successes: register_int_counter_vec_with_registry!(
                opts!("redis_successes_total", "Total Redis operation successes"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            redis_failures: register_int_counter_vec_with_registry!(
                opts!("redis_failures_total", "Total Redis operation failures"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            redis_operation_latency: register_histogram_vec_with_registry!(
                format!("{}_redis_operation_duration_seconds", NAMESPACE),
                "Redis operation latency",
                &[labels::OPERATION_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Redis connection and performance
            redis_connection_pool_size: register_int_counter_vec_with_registry!(
                opts!("redis_connection_pool_size", "Current Redis connection pool size"),
                &[labels::POOL_TYPE],
                registry
            )?,
            redis_memory_usage: register_histogram_vec_with_registry!(
                format!("{}_redis_memory_usage_bytes", NAMESPACE),
                "Redis memory usage in bytes",
                &[labels::MEMORY_TYPE],
                vec![0.0, 1000000.0, 10000000.0, 100000000.0, 1000000000.0, 10000000000.0],
                registry
            )?,
            redis_key_operations: register_int_counter_vec_with_registry!(
                opts!("redis_key_operations_total", "Total Redis key operations"),
                &[labels::KEY_OPERATION_TYPE],
                registry
            )?,
        })
    }
}
