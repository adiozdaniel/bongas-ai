//! Configuration for the analytics module.
//!
//! Provides validated configuration for business metrics collection,
//! export, and retention. Uses builder pattern for ergonomic construction
//! and cross-field validation.

use std::time::Duration;

// ─── Validation Error ───────────────────────────────────────────────────────

/// Validation error for analytics configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsConfigError {
    /// Rate interval must be positive.
    InvalidRateInterval,
    /// Export interval must be positive if export is enabled.
    InvalidExportInterval,
    /// Retention period must be positive if set.
    InvalidRetentionPeriod,
    /// Export interval should be >= rate interval.
    ExportIntervalTooShort,
}

impl std::fmt::Display for AnalyticsConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRateInterval => write!(f, "rate interval must be positive"),
            Self::InvalidExportInterval => {
                write!(f, "export interval must be positive if export is enabled")
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
/// - 1-minute rate calculation interval
/// - Business metrics enabled
/// - No automatic export
/// - No retention period (keep forever)
#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    /// How often to calculate throughput rates.
    rate_interval: Duration,

    /// Whether to track user behavior metrics.
    enable_user_behavior: bool,

    /// Whether to track performance metrics.
    enable_performance: bool,

    /// Whether to track resource usage metrics.
    enable_resource_usage: bool,

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
    pub fn enable_user_behavior(&self) -> bool {
        self.enable_user_behavior
    }

    #[inline]
    pub fn enable_performance(&self) -> bool {
        self.enable_performance
    }

    #[inline]
    pub fn enable_resource_usage(&self) -> bool {
        self.enable_resource_usage
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

    /// Returns true if any tracking is enabled.
    #[inline]
    pub fn tracking_enabled(&self) -> bool {
        self.enable_user_behavior || self.enable_performance || self.enable_resource_usage
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            rate_interval: Duration::from_secs(60), // 1 minute
            enable_user_behavior: true,
            enable_performance: true,
            enable_resource_usage: true,
            export_interval: None,
            retention_period: None,
            namespace: String::from("business"),
        }
    }
}

// ─── Builder ────────────────────────────────────────────────────────────────

/// Builder for `AnalyticsConfig` with validation.
#[derive(Debug, Clone)]
pub struct AnalyticsConfigBuilder {
    rate_interval: Duration,
    enable_user_behavior: bool,
    enable_performance: bool,
    enable_resource_usage: bool,
    export_interval: Option<Duration>,
    retention_period: Option<Duration>,
    namespace: String,
}

impl AnalyticsConfigBuilder {
    fn new() -> Self {
        let defaults = AnalyticsConfig::default();
        Self {
            rate_interval: defaults.rate_interval,
            enable_user_behavior: defaults.enable_user_behavior,
            enable_performance: defaults.enable_performance,
            enable_resource_usage: defaults.enable_resource_usage,
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

    /// Enable or disable user behavior tracking.
    pub fn enable_user_behavior(mut self, enable: bool) -> Self {
        self.enable_user_behavior = enable;
        self
    }

    /// Enable or disable performance tracking.
    pub fn enable_performance(mut self, enable: bool) -> Self {
        self.enable_performance = enable;
        self
    }

    /// Enable or disable resource usage tracking.
    pub fn enable_resource_usage(mut self, enable: bool) -> Self {
        self.enable_resource_usage = enable;
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
            enable_user_behavior: self.enable_user_behavior,
            enable_performance: self.enable_performance,
            enable_resource_usage: self.enable_resource_usage,
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