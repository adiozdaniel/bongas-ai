//! Security layer-specific instrumentation for license validation and binary protection.
//!
//! Provides comprehensive Prometheus metrics for monitoring the complete security
//! enforcement lifecycle, including license validation, hardware fingerprinting,
//! anti-debugging mechanisms, binary integrity verification, and security violation
//! detection. Enables observability of the security subsystem's effectiveness
//! and operational characteristics.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, 
    HistogramVec, IntCounterVec, Registry, opts,
};
use crate::analytics::metrics::{labels, NAMESPACE, DEFAULT_BUCKETS};

/// Metrics collector for security layer operations and enforcement actions.
///
/// Maintains comprehensive metric vectors for:
/// * License validation throughput, reliability, and latency distributions
/// * License lifecycle events including expiration and revocation tracking
/// * Hardware fingerprinting accuracy and performance characteristics
/// * Anti-debugging detection rates and analysis tool identification
/// * Binary integrity verification success and violation patterns
/// * Security heartbeat continuity and system health indicators
/// * Security violation classification and enforcement action severity
pub struct SecurityMetrics {
    // License validation metrics
    /// Total license validation attempts, labeled by security check type.
    pub license_validations: IntCounterVec,
    /// Successful license validations, labeled by security check type.
    pub license_validation_successes: IntCounterVec,
    /// Failed license validations, labeled by security check type.
    pub license_validation_failures: IntCounterVec,
    /// License validation latency distribution, labeled by security check type.
    pub license_validation_latency: HistogramVec,
    
    // License status metrics
    /// Total license status queries, labeled by license status.
    pub license_status: IntCounterVec,
    /// Total license expiration events, labeled by license identifier.
    pub license_expirations: IntCounterVec,
    /// Total license revocation events, labeled by license identifier.
    pub license_revocations: IntCounterVec,
    
    // Hardware fingerprinting metrics
    /// Total hardware fingerprinting attempts, labeled by hardware type.
    pub hardware_fingerprinting_attempts: IntCounterVec,
    /// Successful hardware fingerprinting operations, labeled by hardware type.
    pub hardware_fingerprinting_successes: IntCounterVec,
    /// Failed hardware fingerprinting operations, labeled by hardware type.
    pub hardware_fingerprinting_failures: IntCounterVec,
    /// Hardware fingerprinting latency distribution, labeled by hardware type.
    pub hardware_fingerprinting_latency: HistogramVec,
    
    // Anti-debugging detection metrics
    /// Total anti-debugging checks performed, labeled by check type.
    pub anti_debug_checks: IntCounterVec,
    /// Total debugger detections, labeled by debugger type.
    pub debuggers_detected: IntCounterVec,
    /// Total analysis tool detections, labeled by tool type.
    pub analysis_tools_detected: IntCounterVec,
    /// Anti-debugging check latency distribution, labeled by check type.
    pub anti_debug_latency: HistogramVec,
    
    // Binary integrity metrics
    /// Total binary integrity verification attempts, labeled by check type.
    pub integrity_checks: IntCounterVec,
    /// Total integrity violation detections, labeled by violation type.
    pub integrity_violations: IntCounterVec,
    /// Binary integrity check latency distribution, labeled by check type.
    pub integrity_check_latency: HistogramVec,
    
    // Security heartbeat metrics
    /// Total security heartbeat signals, labeled by heartbeat type.
    pub heartbeat_checks: IntCounterVec,
    /// Successful heartbeat acknowledgments, labeled by heartbeat type.
    pub heartbeat_successes: IntCounterVec,
    /// Failed heartbeat transmissions, labeled by heartbeat type.
    pub heartbeat_failures: IntCounterVec,
    /// Security heartbeat latency distribution, labeled by heartbeat type.
    pub heartbeat_latency: HistogramVec,
    
    // Security violations metrics
    /// Total security violation events, labeled by violation type.
    pub security_violations: IntCounterVec,
    /// Total security enforcement blocks, labeled by block type.
    pub security_blocks: IntCounterVec,
    /// Total security warning emissions, labeled by warning type.
    pub security_warnings: IntCounterVec,
}

impl SecurityMetrics {
    /// Creates and registers all security layer metrics with the provided registry.
    ///
    /// # Arguments
    /// * `registry` - Prometheus registry for metric registration
    ///
    /// # Returns
    /// * `Result<Self, prometheus::Error>` - Initialized security metrics collector or registration error
    ///
    /// # Metric Categories
    /// * License validation and lifecycle management
    /// * Hardware-based device fingerprinting
    /// * Anti-tamper and anti-debugging detection
    /// * Binary and code integrity verification
    /// * Security heartbeat and liveness monitoring
    /// * Violation detection and enforcement actions
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        Ok(Self {
            // License validation metrics
            license_validations: register_int_counter_vec_with_registry!(
                opts!("security_license_validations_total", "Total license validation attempts across all security check types"),
                &[labels::SECURITY_CHECK],
                registry
            )?,
            license_validation_successes: register_int_counter_vec_with_registry!(
                opts!("security_license_validation_successes_total", "Total successful license validations by security check type"),
                &[labels::SECURITY_CHECK],
                registry
            )?,
            license_validation_failures: register_int_counter_vec_with_registry!(
                opts!("security_license_validation_failures_total", "Total failed license validations by security check type"),
                &[labels::SECURITY_CHECK],
                registry
            )?,
            license_validation_latency: register_histogram_vec_with_registry!(
                format!("{}_security_license_validation_duration_seconds", NAMESPACE),
                "License validation latency distribution by security check type",
                &[labels::SECURITY_CHECK],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // License status metrics
            license_status: register_int_counter_vec_with_registry!(
                opts!("security_license_status_total", "Total license status queries by license state"),
                &[labels::LICENSE_STATUS],
                registry
            )?,
            license_expirations: register_int_counter_vec_with_registry!(
                opts!("security_license_expirations_total", "Total license expiration events by license identifier"),
                &[labels::LICENSE_ID],
                registry
            )?,
            license_revocations: register_int_counter_vec_with_registry!(
                opts!("security_license_revocations_total", "Total license revocation events by license identifier"),
                &[labels::LICENSE_ID],
                registry
            )?,
            
            // Hardware fingerprinting metrics
            hardware_fingerprinting_attempts: register_int_counter_vec_with_registry!(
                opts!("security_hardware_fingerprinting_attempts_total", "Total hardware fingerprinting attempts by hardware component type"),
                &[labels::HARDWARE_TYPE],
                registry
            )?,
            hardware_fingerprinting_successes: register_int_counter_vec_with_registry!(
                opts!("security_hardware_fingerprinting_successes_total", "Total successful hardware fingerprinting operations by hardware type"),
                &[labels::HARDWARE_TYPE],
                registry
            )?,
            hardware_fingerprinting_failures: register_int_counter_vec_with_registry!(
                opts!("security_hardware_fingerprinting_failures_total", "Total failed hardware fingerprinting operations by hardware type"),
                &[labels::HARDWARE_TYPE],
                registry
            )?,
            hardware_fingerprinting_latency: register_histogram_vec_with_registry!(
                format!("{}_security_hardware_fingerprinting_duration_seconds", NAMESPACE),
                "Hardware fingerprinting latency distribution by hardware component type",
                &[labels::HARDWARE_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Anti-debugging detection metrics
            anti_debug_checks: register_int_counter_vec_with_registry!(
                opts!("security_anti_debug_checks_total", "Total anti-debugging check executions by detection technique"),
                &[labels::CHECK_TYPE],
                registry
            )?,
            debuggers_detected: register_int_counter_vec_with_registry!(
                opts!("security_debuggers_detected_total", "Total debugger process detections by debugger type"),
                &[labels::DEBUGGER_TYPE],
                registry
            )?,
            analysis_tools_detected: register_int_counter_vec_with_registry!(
                opts!("security_analysis_tools_detected_total", "Total analysis tool detections by tool classification"),
                &[labels::ANALYSIS_TOOL_TYPE],
                registry
            )?,
            anti_debug_latency: register_histogram_vec_with_registry!(
                format!("{}_security_anti_debug_duration_seconds", NAMESPACE),
                "Anti-debugging check latency distribution by detection technique",
                &[labels::CHECK_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Binary integrity metrics
            integrity_checks: register_int_counter_vec_with_registry!(
                opts!("security_integrity_checks_total", "Total binary integrity verification attempts by check type"),
                &[labels::INTEGRITY_CHECK_TYPE],
                registry
            )?,
            integrity_violations: register_int_counter_vec_with_registry!(
                opts!("security_integrity_violations_total", "Total binary integrity violation detections by violation category"),
                &[labels::VIOLATION_TYPE],
                registry
            )?,
            integrity_check_latency: register_histogram_vec_with_registry!(
                format!("{}_security_integrity_check_duration_seconds", NAMESPACE),
                "Binary integrity verification latency distribution by check type",
                &[labels::INTEGRITY_CHECK_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Security heartbeat metrics
            heartbeat_checks: register_int_counter_vec_with_registry!(
                opts!("security_heartbeat_checks_total", "Total security heartbeat signal transmissions by heartbeat type"),
                &[labels::HEARTBEAT_TYPE],
                registry
            )?,
            heartbeat_successes: register_int_counter_vec_with_registry!(
                opts!("security_heartbeat_successes_total", "Total successful security heartbeat acknowledgments by heartbeat type"),
                &[labels::HEARTBEAT_TYPE],
                registry
            )?,
            heartbeat_failures: register_int_counter_vec_with_registry!(
                opts!("security_heartbeat_failures_total", "Total failed security heartbeat transmissions by heartbeat type"),
                &[labels::HEARTBEAT_TYPE],
                registry
            )?,
            heartbeat_latency: register_histogram_vec_with_registry!(
                format!("{}_security_heartbeat_duration_seconds", NAMESPACE),
                "Security heartbeat transmission latency distribution by heartbeat type",
                &[labels::HEARTBEAT_TYPE],
                DEFAULT_BUCKETS.to_vec(),
                registry
            )?,
            
            // Security violations metrics
            security_violations: register_int_counter_vec_with_registry!(
                opts!("security_violations_total", "Total security violation events by violation classification"),
                &[labels::VIOLATION_TYPE],
                registry
            )?,
            security_blocks: register_int_counter_vec_with_registry!(
                opts!("security_blocks_total", "Total security enforcement block actions by block type"),
                &[labels::BLOCK_TYPE],
                registry
            )?,
            security_warnings: register_int_counter_vec_with_registry!(
                opts!("security_warnings_total", "Total security warning emissions by warning category"),
                &[labels::WARNING_TYPE],
                registry
            )?,
        })
    }
}
