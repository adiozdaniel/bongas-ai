//! Metrics registry for centralized circuit breaker metrics storage.
//!
//! Provides thread-safe storage and aggregation of metrics across all
//! circuit breakers in the system.
//!
//! # Netflix Resilience Features
//! - **Lock Poison Recovery**: All locks recover gracefully from panics
//! - **Error Classification Tracking**: Breakdown by classification type
//! - **Degraded/Slow Call Tracking**: Separate counters for degraded responses

use std::sync::{RwLock, PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

use dashmap::DashMap;

use crate::circuit_breaker::CircuitState;
use crate::error::ErrorClassification;

use super::config::AnalyticsConfig;
use super::histogram::HdrHistogram;
use super::types::{
    BreakerSnapshot, ClassificationCounters, ClassificationSnapshot,
    Counter, Gauge, Rate, RegistrySnapshot,
};

// ─── Breaker Metrics ────────────────────────────────────────────────────────

/// Metrics for a single circuit breaker.
pub struct BreakerMetrics {
    pub successes: Counter,
    pub failures: Counter,
    pub timeouts: Counter,
    pub rejections: Counter,
    pub slow_calls: Counter,
    pub degraded_calls: Counter,
    pub latency: HdrHistogram,
    pub concurrent_calls: Gauge,
    pub throughput: Rate,
    state: RwLock<CircuitState>,
    state_changed_at: RwLock<Instant>,
    /// Error breakdown by classification.
    pub classifications: ClassificationCounters,
}

impl BreakerMetrics {
    pub fn new() -> Self {
        Self {
            successes: Counter::new(),
            failures: Counter::new(),
            timeouts: Counter::new(),
            rejections: Counter::new(),
            slow_calls: Counter::new(),
            degraded_calls: Counter::new(),
            latency: HdrHistogram::new(),
            concurrent_calls: Gauge::new(),
            throughput: Rate::new(),
            state: RwLock::new(CircuitState::Closed),
            state_changed_at: RwLock::new(Instant::now()),
            classifications: ClassificationCounters::new(),
        }
    }

    #[inline]
    pub fn total_calls(&self) -> u64 {
        self.successes.get() + self.failures.get() + self.timeouts.get()
    }

    #[inline]
    pub fn failure_rate(&self) -> f64 {
        let total = self.total_calls();
        
        if total == 0 {
            return 0.0;
        }
        (self.failures.get() + self.timeouts.get()) as f64 / total as f64
    }

    /// Record a failure with its classification.
    #[inline]
    pub fn record_failure_classified(&self, classification: ErrorClassification) {
        self.failures.increment();
        self.classifications.record(classification);
    }

    /// Get the current state.
    #[inline]
    pub fn state(&self) -> CircuitState {
        self.read_state()
            .map(|guard| *guard)
            .unwrap_or(CircuitState::Closed)
    }

    /// Get how long the breaker has been in current state.
    #[inline]
    pub fn state_duration(&self) -> std::time::Duration {
        self.read_state_changed_at()
            .map(|guard| guard.elapsed())
            .unwrap_or_default()
    }

    pub fn snapshot(&self, breaker_id: &str) -> BreakerSnapshot {
        let (p50, p90, p95, p99) = self.latency.percentiles();

        BreakerSnapshot {
            breaker_id: breaker_id.to_string(),
            state: format!("{:?}", self.state()),
            total_calls: self.total_calls(),
            successes: self.successes.get(),
            failures: self.failures.get(),
            timeouts: self.timeouts.get(),
            rejections: self.rejections.get(),
            slow_calls: self.slow_calls.get(),
            degraded_calls: self.degraded_calls.get(),
            failure_rate: self.failure_rate(),
            throughput: self.throughput.get(),
            latency_p50_ms: p50 as f64 / 1000.0,
            latency_p90_ms: p90 as f64 / 1000.0,
            latency_p95_ms: p95 as f64 / 1000.0,
            latency_p99_ms: p99 as f64 / 1000.0,
            latency_mean_ms: self.latency.mean() / 1000.0,
            concurrent_calls: self.concurrent_calls.get(),
            state_duration_secs: self.state_duration().as_secs_f64(),
            classifications: self.classifications.snapshot(),
        }
    }

    pub fn update_state(&self, new_state: CircuitState) {
        if let Ok(mut guard) = self.write_state() {
            *guard = new_state;
        }
        if let Ok(mut guard) = self.write_state_changed_at() {
            *guard = Instant::now();
        }
    }

    pub fn reset(&self) {
        self.successes.reset();
        self.failures.reset();
        self.timeouts.reset();
        self.rejections.reset();
        self.slow_calls.reset();
        self.degraded_calls.reset();
        self.latency.reset();
        self.concurrent_calls.reset();
        self.throughput.reset();
        self.classifications.reset();
    }

    // ─── Lock Poison Recovery ──────────────────────────────────────────────

    #[inline]
    fn read_state(&self) -> Result<RwLockReadGuard<'_, CircuitState>, ()> {
        self.state
            .read()
            .or_else(|e| Ok(e.into_inner()))
            .map_err(|_: RwLockReadGuard<'_, CircuitState>| ())
    }

    #[inline]
    fn write_state(&self) -> Result<RwLockWriteGuard<'_, CircuitState>, ()> {
        self.state
            .write()
            .or_else(|e| Ok(e.into_inner()))
            .map_err(|_: RwLockWriteGuard<'_, CircuitState>| ())
    }

    #[inline]
    fn read_state_changed_at(&self) -> Result<RwLockReadGuard<'_, Instant>, ()> {
        self.state_changed_at
            .read()
            .or_else(|e| Ok(e.into_inner()))
            .map_err(|_: RwLockReadGuard<'_, Instant>| ())
    }

    #[inline]
    fn write_state_changed_at(&self) -> Result<RwLockWriteGuard<'_, Instant>, ()> {
        self.state_changed_at
            .write()
            .or_else(|e| Ok(e.into_inner()))
            .map_err(|_: RwLockWriteGuard<'_, Instant>| ())
    }
}

impl Default for BreakerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Metrics Registry ───────────────────────────────────────────────────────

/// Central registry for all circuit breaker metrics.
pub struct MetricsRegistry {
    config: AnalyticsConfig,
    breakers: DashMap<String, BreakerMetrics>,
    last_rate_update: RwLock<Instant>,
}

impl MetricsRegistry {
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            breakers: DashMap::new(),
            last_rate_update: RwLock::new(Instant::now()),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AnalyticsConfig::default())
    }

    /// Get or create metrics for a breaker.
    pub fn get_or_create(
        &self,
        breaker_id: &str,
    ) -> dashmap::mapref::one::Ref<'_, String, BreakerMetrics> {
        // Fast path: already exists
        if let Some(metrics) = self.breakers.get(breaker_id) {
            return metrics;
        }

        // Check max breakers limit
        if self.config.max_breakers() > 0 && self.breakers.len() >= self.config.max_breakers() {
            // Return first breaker as fallback (avoid panic)
            if let Some(entry) = self.breakers.iter().next() {
                if let Some(metrics) = self.breakers.get(entry.key()) {
                    return metrics;
                }
            }
        }

        // Insert new breaker
        self.breakers
            .entry(breaker_id.to_string())
            .or_default();

        // Safe: we just inserted it
        self.breakers
            .get(breaker_id)
            .unwrap_or_else(|| self.breakers.iter().next().map(|e| {
                self.breakers.get(e.key()).unwrap()
            }).unwrap())
    }

    /// Update throughput rates for all breakers.
    pub fn update_rates(&self) {
        let elapsed = {
            let guard = self.read_last_rate_update();
            guard.elapsed()
        };

        if elapsed < self.config.rate_interval() {
            return;
        }

        for entry in self.breakers.iter() {
            entry
                .value()
                .throughput
                .update(entry.value().total_calls(), elapsed);
        }

        if let Ok(mut guard) = self.write_last_rate_update() {
            *guard = Instant::now();
        }
    }

    /// Take a snapshot of all metrics.
    pub fn snapshot(&self) -> RegistrySnapshot {
        self.update_rates();

        let mut snapshots = Vec::with_capacity(self.breakers.len());
        let mut total_calls = 0u64;
        let mut total_successes = 0u64;
        let mut total_failures = 0u64;
        let mut total_timeouts = 0u64;
        let mut total_rejections = 0u64;
        let mut total_slow_calls = 0u64;
        let mut total_degraded = 0u64;
        let mut total_throughput = 0.0f64;

        // Aggregate classification counters
        let mut agg_transient = 0u64;
        let mut agg_permanent = 0u64;
        let mut agg_timeout = 0u64;
        let mut agg_overload = 0u64;
        let mut agg_degraded = 0u64;
        let mut agg_partial = 0u64;

        for entry in self.breakers.iter() {
            let snap = entry.value().snapshot(entry.key());
            total_calls += snap.total_calls;
            total_successes += snap.successes;
            total_failures += snap.failures;
            total_timeouts += snap.timeouts;
            total_rejections += snap.rejections;
            total_slow_calls += snap.slow_calls;
            total_degraded += snap.degraded_calls;
            total_throughput += snap.throughput;

            // Aggregate classifications
            agg_transient += snap.classifications.transient;
            agg_permanent += snap.classifications.permanent;
            agg_timeout += snap.classifications.timeout;
            agg_overload += snap.classifications.overload;
            agg_degraded += snap.classifications.degraded;
            agg_partial += snap.classifications.partial_failure;

            snapshots.push(snap);
        }

        let aggregate_failure_rate = if total_calls > 0 {
            (total_failures + total_timeouts) as f64 / total_calls as f64
        } else {
            0.0
        };

        RegistrySnapshot {
            breakers: snapshots,
            total_calls,
            total_successes,
            total_failures,
            total_timeouts,
            total_rejections,
            total_slow_calls,
            total_degraded,
            aggregate_failure_rate,
            aggregate_throughput: total_throughput,
            aggregate_classifications: ClassificationSnapshot {
                transient: agg_transient,
                permanent: agg_permanent,
                timeout: agg_timeout,
                overload: agg_overload,
                degraded: agg_degraded,
                partial_failure: agg_partial,
            },
        }
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        for entry in self.breakers.iter() {
            entry.value().reset();
        }
    }

    /// Remove a breaker from the registry.
    pub fn remove(&self, breaker_id: &str) {
        self.breakers.remove(breaker_id);
    }

    /// Get the number of registered breakers.
    #[inline]
    pub fn len(&self) -> usize {
        self.breakers.len()
    }

    /// Check if the registry is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.breakers.is_empty()
    }

    /// List all breaker IDs.
    pub fn breaker_ids(&self) -> Vec<String> {
        self.breakers.iter().map(|e| e.key().clone()).collect()
    }

    // ─── Lock Poison Recovery ──────────────────────────────────────────────

    #[inline]
    fn read_last_rate_update(&self) -> RwLockReadGuard<'_, Instant> {
        self.last_rate_update
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn write_last_rate_update(&self) -> Result<RwLockWriteGuard<'_, Instant>, ()> {
        self.last_rate_update
            .write()
            .or_else(|e| Ok(e.into_inner()))
            .map_err(|_: RwLockWriteGuard<'_, Instant>| ())
    }
}
