//! Pipeline configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for pipeline execution resilience including
//! per-stage circuit breaker, bulkhead, timeout, fallback, and analytics.

use std::time::Duration;

/// Pipeline resilience configuration.
///
/// Controls per-stage circuit breaker, timeout, bulkhead, fallback,
/// and analytics behavior for the pipeline executor.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    // ── Per-Stage Circuit Breaker ─────────────────────────────────────────
    pub stage_breaker_enabled: bool,
    pub stage_breaker_failure_rate: f64,
    pub stage_breaker_slow_call_rate: f64,
    pub stage_breaker_slow_call_duration: Duration,
    pub stage_breaker_minimum_calls: u64,
    pub stage_breaker_recovery_timeout: Duration,
    pub stage_breaker_half_open_calls: usize,

    // ── Per-Stage Timeout ────────────────────────────────────────────────
    pub stage_timeout_default: Duration,
    pub fetch_stage_timeout: Duration,
    pub ml_stage_timeout: Duration,
    pub filter_stage_timeout: Duration,

    // ── Pipeline-Level Timeout ───────────────────────────────────────────
    pub pipeline_timeout: Duration,

    // ── Bulkhead (per-stage concurrency) ─────────────────────────────────
    pub stage_max_concurrent: usize,
    pub fetch_max_concurrent: usize,
    pub ml_max_concurrent: usize,

    // ── Fallback ─────────────────────────────────────────────────────────
    pub fallback_enabled: bool,
    pub fallback_on_stage_timeout: bool,
    pub fallback_on_stage_error: bool,
    pub fallback_pass_through_input: bool,

    // ── Analytics ────────────────────────────────────────────────────────
    pub analytics_enabled: bool,
    pub analytics_per_stage: bool,
    pub analytics_sample_rate: f64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            // Per-Stage Circuit Breaker — Netflix Hystrix defaults
            stage_breaker_enabled: true,
            stage_breaker_failure_rate: 0.5,
            stage_breaker_slow_call_rate: 0.5,
            stage_breaker_slow_call_duration: Duration::from_secs(2),
            stage_breaker_minimum_calls: 10,
            stage_breaker_recovery_timeout: Duration::from_secs(30),
            stage_breaker_half_open_calls: 3,

            // Per-Stage Timeout — tuned by stage category
            stage_timeout_default: Duration::from_secs(5),
            fetch_stage_timeout: Duration::from_secs(3),
            ml_stage_timeout: Duration::from_secs(10),
            filter_stage_timeout: Duration::from_secs(2),

            // Pipeline-Level Timeout
            pipeline_timeout: Duration::from_secs(30),

            // Bulkhead — per-stage concurrency limits
            stage_max_concurrent: 32,
            fetch_max_concurrent: 16,
            ml_max_concurrent: 8,

            // Fallback — graceful degradation
            fallback_enabled: true,
            fallback_on_stage_timeout: true,
            fallback_on_stage_error: true,
            fallback_pass_through_input: true,

            // Analytics — observe everything
            analytics_enabled: true,
            analytics_per_stage: true,
            analytics_sample_rate: 1.0,
        }
    }
}

impl PipelineConfig {
    /// Get production-grade defaults (4x dev capacity)
    pub fn production() -> Self {
        Self {
            // Per-Stage Circuit Breaker
            stage_breaker_enabled: true,
            stage_breaker_failure_rate: 0.5,
            stage_breaker_slow_call_rate: 0.5,
            stage_breaker_slow_call_duration: Duration::from_secs(2),
            stage_breaker_minimum_calls: 10,
            stage_breaker_recovery_timeout: Duration::from_secs(30),
            stage_breaker_half_open_calls: 3,

            // Per-Stage Timeout
            stage_timeout_default: Duration::from_secs(5),
            fetch_stage_timeout: Duration::from_secs(3),
            ml_stage_timeout: Duration::from_secs(10),
            filter_stage_timeout: Duration::from_secs(2),

            // Pipeline-Level Timeout
            pipeline_timeout: Duration::from_secs(30),

            // Bulkhead — 4x increase for production
            stage_max_concurrent: 128, // 4x
            fetch_max_concurrent: 64,  // 4x
            ml_max_concurrent: 32,     // 4x

            // Fallback
            fallback_enabled: true,
            fallback_on_stage_timeout: true,
            fallback_on_stage_error: true,
            fallback_pass_through_input: true,

            // Analytics — sample in production to reduce overhead
            analytics_enabled: true,
            analytics_per_stage: true,
            analytics_sample_rate: 0.1, // Sample 10% to reduce overhead
        }
    }
}
