//! Resilience metrics configuration with Builder pattern and validation.

use std::time::Duration;

// ─── Validation Error ───────────────────────────────────────────────────────

/// Validation error for resilience metrics configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResilienceMetricsError {
    InvalidRateInterval,
    InvalidHistogramPrecision,
    InvalidExportInterval,
    InvalidMaxBreakers,
    InvalidRetentionPeriod,
    ExportIntervalTooShort,
}

impl std::fmt::Display for ResilienceMetricsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRateInterval => write!(f, "rate interval must be positive"),
            Self::InvalidHistogramPrecision => write!(f, "histogram precision must be between 1 and 5"),
            Self::InvalidExportInterval => write!(f, "export interval must be positive"),
            Self::InvalidMaxBreakers => write!(f, "max breakers must be non-negative"),
            Self::InvalidRetentionPeriod => write!(f, "retention period must be positive"),
            Self::ExportIntervalTooShort => write!(f, "export interval should be >= rate interval"),
        }
    }
}

impl std::error::Error for ResilienceMetricsError {}

// ─── Configuration ──────────────────────────────────────────────────────────

/// Configuration for the resilience metrics system.
#[derive(Debug, Clone)]
pub struct ResilienceMetricsConfig {
    pub rate_interval: Duration,
    pub enable_histograms: bool,
    pub histogram_precision: u8,
    pub enable_state_duration: bool,
    pub max_breakers: usize,
    pub global_labels: Vec<(String, String)>,
    pub track_error_classifications: bool,
    pub track_slow_calls: bool,
    pub track_degraded: bool,
    pub export_interval: Option<Duration>,
    pub retention_period: Option<Duration>,
    pub namespace: String,
}

impl ResilienceMetricsConfig {
    pub fn builder() -> ResilienceMetricsConfigBuilder {
        ResilienceMetricsConfigBuilder::new()
    }

    #[inline] pub fn rate_interval(&self) -> Duration { self.rate_interval }
    #[inline] pub fn max_breakers(&self) -> usize { self.max_breakers }
}

impl Default for ResilienceMetricsConfig {
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

#[derive(Debug, Clone)]
pub struct ResilienceMetricsConfigBuilder {
    config: ResilienceMetricsConfig,
}

impl ResilienceMetricsConfigBuilder {
    fn new() -> Self {
        Self { config: ResilienceMetricsConfig::default() }
    }

    pub fn rate_interval(mut self, interval: Duration) -> Self {
        self.config.rate_interval = interval;
        self
    }

    pub fn build(self) -> Result<ResilienceMetricsConfig, ResilienceMetricsError> {
        if self.config.rate_interval.is_zero() {
            return Err(ResilienceMetricsError::InvalidRateInterval);
        }
        Ok(self.config)
    }
}
