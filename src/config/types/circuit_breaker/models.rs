//! Circuit breaker configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for Netflix Hystrix-inspired circuit breaker
//! with support for error classification, retry hints, and metrics collection.

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Circuit breaker configuration.
///
/// Configuration for Netflix Hystrix-inspired circuit breaker that protects
/// any async operation from cascading failures. Supports error classification
/// and retry hints for intelligent tripping behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_rate_threshold: f64,
    pub slow_call_rate_threshold: f64,
    pub slow_call_duration: Duration,
    pub minimum_calls: u64,
    pub wait_duration_in_open_state: Duration,
    pub permitted_calls_in_half_open_state: u64,
    pub sliding_window_size: u64,
    pub sliding_window_type: SlidingWindowType,
    pub writable_stack_trace_enabled: bool,
    pub record_exceptions: Vec<String>,
    pub ignore_exceptions: Vec<String>,
}

/// Sliding window type for circuit breaker metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlidingWindowType {
    CountBased,
    TimeBased,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_rate_threshold: 0.5,
            slow_call_rate_threshold: 0.5,
            slow_call_duration: Duration::from_secs(2),
            minimum_calls: 10,
            wait_duration_in_open_state: Duration::from_secs(30),
            permitted_calls_in_half_open_state: 3,
            sliding_window_size: 100,
            sliding_window_type: SlidingWindowType::CountBased,
            writable_stack_trace_enabled: true,
            record_exceptions: vec![
                "RedisError".to_string(),
                "PostgresError".to_string(),
                "IngestionError".to_string(),
            ],
            ignore_exceptions: vec![
                "ValidationError".to_string(),
                "NotFoundError".to_string(),
            ],
        }
    }
}