//! Analytics configuration with Builder pattern and validation.
//!
//! Provides ergonomic, validated configuration for the metrics collection system.
//! All fields are private with accessor methods for proper encapsulation.
//!
//! # Netflix Resilience Features
//! - **Validation**: Cross-field validation for consistent configuration
//! - **Encapsulation**: Private fields with accessors
//! - **Builder Pattern**: Fluent API for configuration

use std::time::Duration;

// ─── Validation Error ───────────────────────────────────────────────────────

/// Validation error for analytics configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsConfigError {
    /// Rate interval must be positive.
    InvalidRateInterval,
    /// Histogram precision must be between 1 and 5.
    InvalidHistogramPrecision,
    /// Export interval must be positive if export is enabled.
    InvalidExportInterval,
    /// Max breakers cannot be negative (0 means unlimited).
    InvalidMaxBreakers,
    /// Retention period must be positive if set.
    InvalidRetentionPeriod,
    /// Export interval should be >= rate interval.
    ExportIntervalTooShort,
}

impl std::fmt::Display for AnalyticsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRateInterval => write!(f, "rate interval must be positive"),
            Self::InvalidHistogramPrecision => {
                write!(f, "histogram precision must be between 1 and 5")
            }
            Self::InvalidExportInterval => {
                write!(f, "export interval must be positive if export is enabled")
            }
            Self::InvalidMaxBreakers => {
                write!(f, "max breakers must be non-negative (0 = unlimited)")
            }
            Self::InvalidRetentionPeriod => {
                write!(f, "retention period must be positive if set")
            }
            Self::ExportIntervalTooShort => {
                write!(f, "export interval should be >= rate interval")
            }
        }
    }
}

impl std::error::Error for AnalyticsConfigError {}

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for the analytics system.
///
/// All fields are private for encapsulation. Use `AnalyticsConfig::builder()`
/// for construction and accessor methods for reading values.
///
/// # Defaults
/// - 1-second rate calculation interval
/// - Histograms enabled with precision 3
/// - State duration tracking enabled
/// - Unlimited breakers (0)
/// - No automatic export
#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    /// How often to calculate throughput rates.
    rate_interval: Duration,

    /// Whether to track per-breaker histograms.
    enable_histograms: bool,

    /// Histogram precision (1-5, higher = more memory, more accuracy).
    histogram_precision: u8,

    /// Whether to track state duration.
    enable_state_duration: bool,

    /// Maximum number of breakers to track (0 = unlimited).
    max_breakers: usize,

    /// Labels to attach to all metrics.
    global_labels: Vec<(String, String)>,

    /// Whether to track metrics by error classification.
    track_error_classifications: bool,

    /// Whether to track slow calls separately.
    track_slow_calls: bool,

    /// Whether to track degraded responses.
    track_degraded: bool,

    /// Optional export interval for automatic export.
    export_interval: Option<Duration>,

    /// Retention period for metrics (None = keep forever).
    retention_period: Option<Duration>,

    /// Namespace prefix for exported metrics.
    namespace: String,
}

impl AnalyticsConfig {
    /// Start building a configuration.
    pub fn builder() -> AnalyticsConfigBuilder {
        AnalyticsConfigBuilder::new()
    }

    // ─── Accessors ─────────────────────────────────────────────────────────

    #[inline]
    pub fn rate_interval(&self) -> Duration {
        self.rate_interval
    }

    #[inline]
    pub fn enable_histograms(&self) -> bool {
        self.enable_histograms
    }

    #[inline]
    pub fn histogram_precision(&self) -> u8 {
        self.histogram_precision
    }

    #[inline]
    pub fn enable_state_duration(&self) -> bool {
        self.enable_state_duration
    }

    #[inline]
    pub fn max_breakers(&self) -> usize {
        self.max_breakers
    }

    #[inline]
    pub fn global_labels(&self) -> &[(String, String)] {
        &self.global_labels
    }

    #[inline]
    pub fn track_error_classifications(&self) -> bool {
        self.track_error_classifications
    }

    #[inline]
    pub fn track_slow_calls(&self) -> bool {
        self.track_slow_calls
    }

    #[inline]
    pub fn track_degraded(&self) -> bool {
        self.track_degraded
    }

    #[inline]
    pub fn export_interval(&self) -> Option<Duration> {
        self.export_interval
    }

    #[inline]
    pub fn retention_period(&self) -> Option<Duration> {
        self.retention_period
    }

    #[inline]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Returns true if any advanced tracking is enabled.
    #[inline]
    pub fn advanced_tracking_enabled(&self) -> bool {
        self.track_error_classifications || self.track_slow_calls || self.track_degraded
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            rate_interval: Duration::from_secs(1),
            enable_histograms: true,
            histogram_precision: 3,
            enable_state_duration: true,
            max_breakers: 0,
            global_labels: Vec::new(),
            track_error_classifications: true,
            track_slow_calls: true,
            track_degraded: true,
            export_interval: None,
            retention_period: None,
            namespace: String::from("resilience"),
        }
    }
}

// ─── Builder ────────────────────────────────────────────────────────────────

/// Builder for `AnalyticsConfig` with validation.
#[derive(Debug, Clone)]
pub struct AnalyticsConfigBuilder {
    rate_interval: Duration,
    enable_histograms: bool,
    histogram_precision: u8,
    enable_state_duration: bool,
    max_breakers: usize,
    global_labels: Vec<(String, String)>,
    track_error_classifications: bool,
    track_slow_calls: bool,
    track_degraded: bool,
    export_interval: Option<Duration>,
    retention_period: Option<Duration>,
    namespace: String,
}

impl AnalyticsConfigBuilder {
    fn new() -> Self {
        let defaults = AnalyticsConfig::default();
        Self {
            rate_interval: defaults.rate_interval,
            enable_histograms: defaults.enable_histograms,
            histogram_precision: defaults.histogram_precision,
            enable_state_duration: defaults.enable_state_duration,
            max_breakers: defaults.max_breakers,
            global_labels: defaults.global_labels,
            track_error_classifications: defaults.track_error_classifications,
            track_slow_calls: defaults.track_slow_calls,
            track_degraded: defaults.track_degraded,
            export_interval: defaults.export_interval,
            retention_period: defaults.retention_period,
            namespace: defaults.namespace,
        }
    }

    /// Set how often to calculate throughput rates.
    pub fn rate_interval(mut self, interval: Duration) -> Self {
        self.rate_interval = interval;
        self
    }

    /// Enable or disable histogram tracking.
    pub fn enable_histograms(mut self, enable: bool) -> Self {
        self.enable_histograms = enable;
        self
    }

    /// Set histogram precision (1-5).
    pub fn histogram_precision(mut self, precision: u8) -> Self {
        self.histogram_precision = precision;
        self
    }

    /// Enable or disable state duration tracking.
    pub fn enable_state_duration(mut self, enable: bool) -> Self {
        self.enable_state_duration = enable;
        self
    }

    /// Set maximum number of breakers to track (0 = unlimited).
    pub fn max_breakers(mut self, max: usize) -> Self {
        self.max_breakers = max;
        self
    }

    /// Add a global label to all metrics.
    pub fn global_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.global_labels.push((key.into(), value.into()));
        self
    }

    /// Enable or disable error classification tracking.
    pub fn track_error_classifications(mut self, enable: bool) -> Self {
        self.track_error_classifications = enable;
        self
    }

    /// Enable or disable slow call tracking.
    pub fn track_slow_calls(mut self, enable: bool) -> Self {
        self.track_slow_calls = enable;
        self
    }

    /// Enable or disable degraded response tracking.
    pub fn track_degraded(mut self, enable: bool) -> Self {
        self.track_degraded = enable;
        self
    }

    /// Set automatic export interval.
    pub fn export_interval(mut self, interval: Duration) -> Self {
        self.export_interval = Some(interval);
        self
    }

    /// Set metrics retention period.
    pub fn retention_period(mut self, period: Duration) -> Self {
        self.retention_period = Some(period);
        self
    }

    /// Set namespace prefix for exported metrics.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), AnalyticsConfigError> {
        // Individual field validation
        if self.rate_interval.is_zero() {
            return Err(AnalyticsConfigError::InvalidRateInterval);
        }

        if self.histogram_precision < 1 || self.histogram_precision > 5 {
            return Err(AnalyticsConfigError::InvalidHistogramPrecision);
        }

        if let Some(export_interval) = self.export_interval {
            if export_interval.is_zero() {
                return Err(AnalyticsConfigError::InvalidExportInterval);
            }
        }

        if let Some(retention) = self.retention_period {
            if retention.is_zero() {
                return Err(AnalyticsConfigError::InvalidRetentionPeriod);
            }
        }

        // Cross-field validation
        if let Some(export_interval) = self.export_interval {
            if export_interval < self.rate_interval {
                return Err(AnalyticsConfigError::ExportIntervalTooShort);
            }
        }

        Ok(())
    }

    /// Build the configuration, returning an error if validation fails.
    pub fn build(self) -> Result<AnalyticsConfig, AnalyticsConfigError> {
        self.validate()?;

        Ok(AnalyticsConfig {
            rate_interval: self.rate_interval,
            enable_histograms: self.enable_histograms,
            histogram_precision: self.histogram_precision,
            enable_state_duration: self.enable_state_duration,
            max_breakers: self.max_breakers,
            global_labels: self.global_labels,
            track_error_classifications: self.track_error_classifications,
            track_slow_calls: self.track_slow_calls,
            track_degraded: self.track_degraded,
            export_interval: self.export_interval,
            retention_period: self.retention_period,
            namespace: self.namespace,
        })
    }
}

impl Default for AnalyticsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
