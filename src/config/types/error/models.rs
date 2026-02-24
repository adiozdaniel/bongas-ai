//! Error handling configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for error classification, retry strategies,
//! and error context management for Netflix-grade resilience patterns.

use std::time::Duration;

/// Error handling configuration.
///
/// Configuration for comprehensive error handling with classification,
/// retry strategies, and context management for all domain errors.
#[derive(Debug, Clone)]
pub struct ErrorConfig {
    pub enabled: bool,
    pub retry_enabled: bool,
    pub max_retries: u32,
    pub retry_backoff_strategy: BackoffStrategy,
    pub retry_jitter_enabled: bool,
    pub retry_timeout: Duration,
    pub error_context_enabled: bool,
    pub error_context_max_length: usize,
    pub transient_error_patterns: Vec<String>,
    pub permanent_error_patterns: Vec<String>,
    pub timeout_error_patterns: Vec<String>,
    pub overload_error_patterns: Vec<String>,
    pub degraded_error_patterns: Vec<String>,
    pub partial_failure_patterns: Vec<String>,
}

/// Backoff strategy for retry operations.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    Fixed(Duration),
    Exponential { base: Duration, max_delay: Duration },
    Linear { increment: Duration, max_delay: Duration },
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retry_enabled: true,
            max_retries: 3,
            retry_backoff_strategy: BackoffStrategy::Exponential {
                base: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
            },
            retry_jitter_enabled: true,
            retry_timeout: Duration::from_secs(30),
            error_context_enabled: true,
            error_context_max_length: 1000,
            transient_error_patterns: vec![
                "Connection refused".to_string(),
                "Timeout".to_string(),
                "Network error".to_string(),
            ],
            permanent_error_patterns: vec![
                "Invalid credentials".to_string(),
                "Not found".to_string(),
                "Permission denied".to_string(),
            ],
            timeout_error_patterns: vec![
                "Timeout".to_string(),
                "Request timeout".to_string(),
                "Connection timeout".to_string(),
            ],
            overload_error_patterns: vec![
                "Too many requests".to_string(),
                "Rate limit exceeded".to_string(),
                "Service unavailable".to_string(),
            ],
            degraded_error_patterns: vec![
                "Degraded performance".to_string(),
                "High latency".to_string(),
                "Resource exhausted".to_string(),
            ],
            partial_failure_patterns: vec![
                "Partial failure".to_string(),
                "Some operations failed".to_string(),
                "Mixed results".to_string(),
            ],
        }
    }
}