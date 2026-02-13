//! Staging operations-specific instrumentation for multi-level cache coordination.
//!
//! Provides comprehensive Prometheus metrics for monitoring staging layer operations,
//! including L1/L2 cache coordination, invalidation propagation, staleness detection,
//! and operational performance of the staging management subsystem. Enables observability
//! of cache efficiency and data freshness guarantees.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, 
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for staging operations and multi-level cache coordination.
///
/// Maintains comprehensive metric vectors for:
/// * Multi-level cache hit/miss ratios and eviction patterns
/// * Cache invalidation throughput, reasons, and propagation latency
/// * Staleness detection accuracy and check performance
/// * Staging manager operation success rates and response times
/// * Cross-cache consistency and synchronization effectiveness
pub struct StagingMetrics {
    // Staging cache operations (L1 + L2 coordination)
    /// Total staging cache hits, labeled by cache type (L1/L2).
    pub staging_cache_hits: IntCounterVec,
    /// Total staging cache misses, labeled by cache type (L1/L2).
    pub staging_cache_misses: IntCounterVec,
    /// Total staging cache evictions, labeled by cache type (L1/L2).
    pub staging_cache_evictions: IntCounterVec,
    /// Staging cache lookup latency distribution, labeled by cache type.
    pub staging_cache_lookup_latency: HistogramVec,
    
    // Cache invalidation events
    /// Total cache invalidation events, labeled by scenario and invalidation reason.
    pub invalidations: IntCounterVec,
    /// Total invalidation reason occurrences, labeled by invalidation reason.
    pub invalidation_reasons: IntCounterVec,
    /// Cache invalidation propagation latency, labeled by scenario.
    pub invalidation_latency: HistogramVec,
    
    // Cache staleness tracking
    /// Total staleness verification checks, labeled by scenario.
    pub staleness_checks: IntCounterVec,
    /// Total staleness detection events, labeled by scenario.
    pub staleness_events: IntCounterVec,
    /// Staleness tracking operation latency, labeled by scenario.
    pub staleness_latency: HistogramVec,
    
    // Staging manager performance
    /// Total staging manager operations, labeled by operation type.
    pub staging_operations: IntCounterVec,
    /// Successful staging manager operations, labeled by operation type.
    pub staging_successes: IntCounterVec,
    /// Failed staging manager operations, labeled by operation type.
    pub staging_failures: IntCounterVec,
    /// Staging manager operation latency distribution, labeled by operation type.
    pub staging_operation_latency: HistogramVec,
}

impl StagingMetrics {
    /// Creates and registers all staging operation metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized staging metrics collector or registration error
    ///
    /// # Metric Categories
    /// * Multi-level cache performance and efficiency
    /// * Invalidation propagation and coordination
    /// * Data freshness and staleness detection
    /// * Operational reliability and throughput
    /// * Cross-cache synchronization patterns
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // Staging cache operations (L1 + L2 coordination)
            staging_cache_hits: register_int_counter_vec_with_registry!(
                opts!("staging_cache_hits_total", "Total staging cache hit events by cache level, indicating successful data retrieval from L1 or L2 cache"),
                &[labels::CACHE_TYPE],
                registry
            )?,
            staging_cache_misses: register_int_counter_vec_with_registry!(
                opts!("staging_cache_misses_total", "Total staging cache miss events by cache level, requiring fallback to next cache tier or backing store"),
                &[labels::CACHE_TYPE],
                registry
            )?,
            staging_cache_evictions: register_int_counter_vec_with_registry!(
                opts!("staging_cache_evictions_total", "Total staging cache eviction events by cache level, tracking cache pressure and replacement policy effectiveness"),
                &[labels::CACHE_TYPE],
                registry
            )?,
            staging_cache_lookup_latency: register_histogram_vec_with_registry!(
                format!("{}_staging_cache_lookup_duration_seconds", NAMESPACE),
                "Staging cache lookup latency distribution by cache level, measuring retrieval performance",
                &[labels::CACHE_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Cache invalidation events
            invalidations: register_int_counter_vec_with_registry!(
                opts!("staging_invalidations_total", "Total staging cache invalidation events by scenario and invalidation reason, tracking cache consistency operations"),
                &[labels::SCENARIO, labels::INVALIDATION_REASON],
                registry
            )?,
            invalidation_reasons: register_int_counter_vec_with_registry!(
                opts!("staging_invalidation_reasons_total", "Total staging cache invalidation reason occurrences, identifying primary drivers of cache invalidation"),
                &[labels::INVALIDATION_REASON],
                registry
            )?,
            invalidation_latency: register_histogram_vec_with_registry!(
                format!("{}_staging_invalidation_duration_seconds", NAMESPACE),
                "Staging cache invalidation propagation latency distribution by scenario, measuring time to achieve cross-cache consistency",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Cache staleness tracking
            staleness_checks: register_int_counter_vec_with_registry!(
                opts!("staging_staleness_checks_total", "Total staging staleness verification checks by scenario, measuring proactive data freshness monitoring"),
                &[labels::SCENARIO],
                registry
            )?,
            staleness_events: register_int_counter_vec_with_registry!(
                opts!("staging_staleness_events_total", "Total staging staleness detection events by scenario, tracking frequency of outdated data identification"),
                &[labels::SCENARIO],
                registry
            )?,
            staleness_latency: register_histogram_vec_with_registry!(
                format!("{}_staging_staleness_duration_seconds", NAMESPACE),
                "Staging staleness tracking operation latency distribution by scenario, measuring detection and verification performance",
                &[labels::SCENARIO],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Staging manager performance
            staging_operations: register_int_counter_vec_with_registry!(
                opts!("staging_operations_total", "Total staging manager operation attempts by operation type, tracking overall subsystem workload"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            staging_successes: register_int_counter_vec_with_registry!(
                opts!("staging_successes_total", "Total successful staging manager operations by operation type, measuring subsystem reliability"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            staging_failures: register_int_counter_vec_with_registry!(
                opts!("staging_failures_total", "Total failed staging manager operations by operation type, identifying error patterns and failure modes"),
                &[labels::OPERATION_TYPE],
                registry
            )?,
            staging_operation_latency: register_histogram_vec_with_registry!(
                format!("{}_staging_operation_duration_seconds", NAMESPACE),
                "Staging manager operation latency distribution by operation type, measuring operational performance and throughput capacity",
                &[labels::OPERATION_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
        })
    }
}
