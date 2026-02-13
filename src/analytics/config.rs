//! Analytics configuration with Builder pattern.
//!
//! Provides ergonomic configuration for the metrics collection system.

use std::time::Duration;

/// Configuration for the analytics system.
#[derive(Debug, Clone)]
pub struct AnalyticsConfig {
    /// How often to calculate throughput rates.
    pub rate_interval: Duration,

    /// Whether to track per-breaker histograms.
    pub enable_histograms: bool,

    /// Whether to track state duration.
    pub enable_state_duration: bool,

    /// Maximum number of breakers to track (0 = unlimited).
    pub max_breakers: usize,

    /// Labels to attach to all metrics.
    pub global_labels: Vec<(String, String)>,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            rate_interval: Duration::from_secs(1),
            enable_histograms: true,
            enable_state_duration: true,
            max_breakers: 0,
            global_labels: Vec::new(),
        }
    }
}

impl AnalyticsConfig {
    pub fn builder() -> AnalyticsConfigBuilder {
        AnalyticsConfigBuilder::new()
    }
}

/// Builder for `AnalyticsConfig`.
pub struct AnalyticsConfigBuilder {
    config: AnalyticsConfig,
}

impl AnalyticsConfigBuilder {
    fn new() -> Self {
        Self {
            config: AnalyticsConfig::default(),
        }
    }

    pub fn rate_interval(mut self, interval: Duration) -> Self {
        self.config.rate_interval = interval;
        self
    }

    pub fn enable_histograms(mut self, enable: bool) -> Self {
        self.config.enable_histograms = enable;
        self
    }

    pub fn enable_state_duration(mut self, enable: bool) -> Self {
        self.config.enable_state_duration = enable;
        self
    }

    pub fn max_breakers(mut self, max: usize) -> Self {
        self.config.max_breakers = max;
        self
    }

    pub fn global_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.global_labels.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> AnalyticsConfig {
        self.config
    }
}
