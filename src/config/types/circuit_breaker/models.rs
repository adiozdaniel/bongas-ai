//! Circuit breaker configuration for the Composite Configuration Pattern.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::circuit_breaker::config::builder::CircuitBreakerConfigBuilder;

/// Circuit breaker configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_rate_threshold: f64,
    pub minimum_calls: u64,
    pub recovery_timeout: Duration,
    pub half_open_max_calls: usize,
    pub slow_call_rate_threshold: f64,
    pub slow_call_duration: Duration,
    pub sliding_window_type: SlidingWindowType,
    pub sliding_window_size: usize,
    pub call_timeout: Duration,
    pub max_concurrent_calls: usize,
    pub consecutive_failure_threshold: Option<u64>,
    pub bulkhead_enabled: bool,
    pub bulkhead_per_endpoint: bool,
    
    // Hystrix / Loader compatibility fields
    pub wait_duration_in_open_state: Option<Duration>,
    pub permitted_calls_in_half_open_state: Option<u64>,
    pub writable_stack_trace_enabled: bool,
    pub record_exceptions: Vec<String>,
    pub ignore_exceptions: Vec<String>,
}

/// Type of sliding window used for statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlidingWindowType {
    Count,
    Time,
    CountBased,
    TimeBased,
}

impl CircuitBreakerConfig {
    pub fn builder() -> CircuitBreakerConfigBuilder {
        CircuitBreakerConfigBuilder::new()
    }

    #[inline] pub fn failure_rate_threshold(&self) -> f64 { self.failure_rate_threshold }
    #[inline] pub fn minimum_calls(&self) -> u64 { self.minimum_calls }
    #[inline] pub fn recovery_timeout(&self) -> Duration { 
        self.wait_duration_in_open_state.unwrap_or(self.recovery_timeout) 
    }
    #[inline] pub fn half_open_max_calls(&self) -> usize { 
        self.permitted_calls_in_half_open_state.map(|c| c as usize).unwrap_or(self.half_open_max_calls)
    }
    #[inline] pub fn slow_call_rate_threshold(&self) -> Option<f64> { Some(self.slow_call_rate_threshold) }
    #[inline] pub fn slow_call_duration(&self) -> Option<Duration> { Some(self.slow_call_duration) }
    #[inline] pub fn call_timeout(&self) -> Option<Duration> { Some(self.call_timeout) }
    #[inline] pub fn max_concurrent_calls(&self) -> usize { self.max_concurrent_calls }
    #[inline] pub fn consecutive_failure_threshold(&self) -> Option<u64> { self.consecutive_failure_threshold }
    #[inline] pub fn window_duration(&self) -> Duration { self.recovery_timeout }
    #[inline] pub fn bucket_count(&self) -> usize { 10 }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_rate_threshold: 0.5,
            minimum_calls: 10,
            recovery_timeout: Duration::from_secs(60),
            half_open_max_calls: 10,
            slow_call_rate_threshold: 1.0,
            slow_call_duration: Duration::from_secs(60),
            sliding_window_type: SlidingWindowType::Count,
            sliding_window_size: 100,
            call_timeout: Duration::from_secs(30),
            max_concurrent_calls: 0,
            consecutive_failure_threshold: None,
            bulkhead_enabled: true,
            bulkhead_per_endpoint: true,
            wait_duration_in_open_state: None,
            permitted_calls_in_half_open_state: None,
            writable_stack_trace_enabled: false,
            record_exceptions: vec![],
            ignore_exceptions: vec![],
        }
    }
}
