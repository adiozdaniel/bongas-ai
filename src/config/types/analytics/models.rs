//! Analytics configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for high-throughput metrics collection, aggregation,
//! and export for all circuit breakers in the system with HDR histograms.

use std::time::Duration;

/// Analytics configuration.
///
/// Configuration for high-throughput metrics collection, aggregation,
/// and export for all circuit breakers with HDR histograms for accurate
/// latency percentiles and error classification tracking.
#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    pub enabled: bool,
    pub metrics_collection_interval: Duration,
    pub histogram_precision: u32,
    pub histogram_max_value: u64,
    pub histogram_min_value: u64,
    pub export_format: ExportFormat,
    pub export_interval: Duration,
    pub export_path: String,
    pub error_classification_enabled: bool,
    pub degraded_failure_tracking_enabled: bool,
    pub partial_failure_tracking_enabled: bool,
    pub lock_poison_recovery_enabled: bool,
    pub validated_config_enabled: bool,
    pub max_metrics_history: usize,
}

/// Export format for analytics data.
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Prometheus,
    Csv,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_collection_interval: Duration::from_secs(10),
            histogram_precision: 3,
            histogram_max_value: 60_000_000, // 60 seconds in microseconds
            histogram_min_value: 1,
            export_format: ExportFormat::Json,
            export_interval: Duration::from_secs(60),
            export_path: "./metrics".to_string(),
            error_classification_enabled: true,
            degraded_failure_tracking_enabled: true,
            partial_failure_tracking_enabled: true,
            lock_poison_recovery_enabled: true,
            validated_config_enabled: true,
            max_metrics_history: 1000,
        }
    }
}