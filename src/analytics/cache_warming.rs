//! Cache warming-specific instrumentation.
//!
//! Provides Prometheus metrics for monitoring and analyzing cache warming operations.
//! Tracks warming attempts, success rates, latency distributions, strategy effectiveness,
//! and popular user targeting. All metrics are registered with a provided registry
//! for integration with the main metrics collection system.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for cache warming operations.
///
/// Maintains counter and histogram vectors that track:
/// * Warming operation throughput and success rates
/// * Operation latency percentiles
/// * Cache hit/miss effectiveness post-warming
/// * Strategy distribution and performance
/// * Popular user targeting efficiency
pub struct CacheWarmingMetrics {
    /// Total number of cache warming attempts, labeled by scenario.
    pub warming_attempts: IntCounterVec,
    /// Successful cache warming operations, labeled by scenario.
    pub warming_successes: IntCounterVec,
    /// Failed cache warming operations, labeled by scenario.
    pub warming_failures: IntCounterVec,
    /// Duration of cache warming operations, labeled by scenario.
    pub warming_latency: HistogramVec,

    /// Cache hits during warmed data access, labeled by scenario.
    pub warming_cache_hits: IntCounterVec,
    /// Cache misses during warmed data access, labeled by scenario.
    pub warming_cache_misses: IntCounterVec,
    /// Hit rate distribution for warmed cache entries, labeled by scenario.
    pub warming_hit_rate: HistogramVec,

    /// Number of distinct cache warming targets, labeled by target type.
    pub warming_targets: IntCounterVec,
    /// Number of times each warming strategy was employed.
    pub warming_strategies: IntCounterVec,
    /// Duration of each warming strategy execution.
    pub warming_strategy_latency: HistogramVec,

    /// Total popular user warming operations, labeled by user type.
    pub popular_user_warming: IntCounterVec,
    /// Successful popular user warming operations, labeled by user type.
    pub popular_user_warming_successes: IntCounterVec,
    /// Failed popular user warming operations, labeled by user type.
    pub popular_user_warming_failures: IntCounterVec,
    /// Duration of popular user warming operations, labeled by user type.
    pub popular_user_warming_latency: HistogramVec,
}

impl CacheWarmingMetrics {
    /// Creates and registers all cache warming metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized metrics collector or registration error
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Cache warming operations
            warming_attempts: register_int_counter_vec_with_registry!(
                opts!("cache_warming_attempts_total", "Total cache warming attempts"),
                &[labels::SCENARIO],
                registry
            )?,
            warming_successes: register_int_counter_vec_with_registry!(
                opts!("cache_warming_successes_total", "Total cache warming successes"),
                &[labels::SCENARIO],
                registry
            )?,
            warming_failures: register_int_counter_vec_with_registry!(
                opts!("cache_warming_failures_total", "Total cache warming failures"),
                &[labels::SCENARIO],
                registry
            )?,
            warming_latency: register_histogram_vec_with_registry!(
                format!("{}_cache_warming_duration_seconds", NAMESPACE),
                "Cache warming latency",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Warming effectiveness
            warming_cache_hits: register_int_counter_vec_with_registry!(
                opts!("cache_warming_cache_hits_total", "Total cache warming cache hits"),
                &[labels::SCENARIO],
                registry
            )?,
            warming_cache_misses: register_int_counter_vec_with_registry!(
                opts!("cache_warming_cache_misses_total", "Total cache warming cache misses"),
                &[labels::SCENARIO],
                registry
            )?,
            warming_hit_rate: register_histogram_vec_with_registry!(
                format!("{}_cache_warming_hit_rate", NAMESPACE),
                "Cache warming hit rate",
                &[labels::SCENARIO],
                vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0],
                registry
            )?,

            // Warming targets and strategies
            warming_targets: register_int_counter_vec_with_registry!(
                opts!("cache_warming_targets_total", "Total cache warming targets"),
                &[labels::CACHE_TYPE],
                registry
            )?,
            warming_strategies: register_int_counter_vec_with_registry!(
                opts!("cache_warming_strategies_total", "Total cache warming strategies used"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            warming_strategy_latency: register_histogram_vec_with_registry!(
                format!("{}_cache_warming_strategy_duration_seconds", NAMESPACE),
                "Cache warming strategy latency",
                &[labels::OPERATION_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,

            // Popular user warming
            popular_user_warming: register_int_counter_vec_with_registry!(
                opts!("popular_user_warming_total", "Total popular user warming operations"),
                &[labels::USER_TYPE],
                registry
            )?,
            popular_user_warming_successes: register_int_counter_vec_with_registry!(
                opts!("popular_user_warming_successes_total", "Total popular user warming successes"),
                &[labels::USER_TYPE],
                registry
            )?,
            popular_user_warming_failures: register_int_counter_vec_with_registry!(
                opts!("popular_user_warming_failures_total", "Total popular user warming failures"),
                &[labels::USER_TYPE],
                registry
            )?,
            popular_user_warming_latency: register_histogram_vec_with_registry!(
                format!("{}_popular_user_warming_duration_seconds", NAMESPACE),
                "Popular user warming latency",
                &[labels::USER_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
