//! Metrics registry for centralized circuit breaker metrics storage.
//!
//! Provides thread-safe storage and aggregation of metrics across all
//! circuit breakers in the system.

use std::sync::RwLock;
use std::time::Instant;

use dashmap::DashMap;

use crate::circuit_breaker::CircuitState;

use super::config::AnalyticsConfig;
use super::histogram::HdrHistogram;
use super::types::{BreakerSnapshot, Counter, Gauge, Rate, RegistrySnapshot};

/// Metrics for a single circuit breaker.
pub struct BreakerMetrics {
    pub successes: Counter,
    pub failures: Counter,
    pub timeouts: Counter,
    pub rejections: Counter,
    pub latency: HdrHistogram,
    pub concurrent_calls: Gauge,
    pub throughput: Rate,
    pub state: RwLock<CircuitState>,
    pub state_changed_at: RwLock<Instant>,
}

impl BreakerMetrics {
    pub fn new() -> Self {
        Self {
            successes: Counter::new(),
            failures: Counter::new(),
            timeouts: Counter::new(),
            rejections: Counter::new(),
            latency: HdrHistogram::new(),
            concurrent_calls: Gauge::new(),
            throughput: Rate::new(),
            state: RwLock::new(CircuitState::Closed),
            state_changed_at: RwLock::new(Instant::now()),
        }
    }

    pub fn total_calls(&self) -> u64 {
        self.successes.get() + self.failures.get() + self.timeouts.get()
    }

    pub fn failure_rate(&self) -> f64 {
        let total = self.total_calls();
        if total == 0 {
            return 0.0;
        }
        (self.failures.get() + self.timeouts.get()) as f64 / total as f64
    }

    pub fn snapshot(&self, breaker_id: &str) -> BreakerSnapshot {
        let (p50, p90, p95, p99) = self.latency.percentiles();

        let state = self.state.read()
            .map(|guard| *guard)
            .unwrap_or(CircuitState::Closed);

        let state_duration = self.state_changed_at.read()
            .map(|guard| guard.elapsed())
            .unwrap_or_default();

        BreakerSnapshot {
            breaker_id: breaker_id.to_string(),
            state: format!("{:?}", state),
            total_calls: self.total_calls(),
            successes: self.successes.get(),
            failures: self.failures.get(),
            timeouts: self.timeouts.get(),
            rejections: self.rejections.get(),
            failure_rate: self.failure_rate(),
            throughput: self.throughput.get(),
            latency_p50_ms: p50 as f64 / 1000.0,
            latency_p90_ms: p90 as f64 / 1000.0,
            latency_p95_ms: p95 as f64 / 1000.0,
            latency_p99_ms: p99 as f64 / 1000.0,
            latency_mean_ms: self.latency.mean() / 1000.0,
            concurrent_calls: self.concurrent_calls.get(),
            state_duration_secs: state_duration.as_secs_f64(),
        }
    }

    pub fn update_state(&self, new_state: CircuitState) {
        if let Ok(mut guard) = self.state.write() {
            *guard = new_state;
        }
        if let Ok(mut guard) = self.state_changed_at.write() {
            *guard = Instant::now();
        }
    }
}

impl Default for BreakerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Central registry for all circuit breaker metrics.
pub struct MetricsRegistry {
    config: AnalyticsConfig,
    breakers: DashMap<String, BreakerMetrics>,
    last_rate_update: RwLock<Instant>
}

impl MetricsRegistry {
    pub fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            breakers: DashMap::new(),
            last_rate_update: RwLock::new(Instant::now())
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(AnalyticsConfig::default())
    }

    /// Get or create metrics for a breaker.
    pub fn get_or_create(&self, breaker_id: &str) -> dashmap::mapref::one::Ref<'_, String, BreakerMetrics> {
        // Fast path: already exists
        if let Some(metrics) = self.breakers.get(breaker_id) {
            return metrics;
        }

        // Check max breakers limit
        if self.config.max_breakers > 0
            && self.breakers.len() >= self.config.max_breakers
        {
            // Return first breaker as fallback
            if let Some(entry) = self.breakers.iter().next() {
                return self.breakers.get(entry.key()).expect("Bug: breaker from iter not found");
            }
        }

        // Insert new breaker
        self.breakers.entry(breaker_id.to_string()).or_insert_with(|| {
            BreakerMetrics::new()
        });

        self.breakers.get(breaker_id).expect("Bug: just inserted breaker not found")
    }

    /// Update throughput rates for all breakers.
    pub fn update_rates(&self) {
        let elapsed = {
            let Ok(guard) = self.last_rate_update.read() else {
                return;
            };
            guard.elapsed()
        };

        if elapsed < self.config.rate_interval {
            return;
        }

        for entry in self.breakers.iter() {
            entry.value().throughput.update(entry.value().total_calls(), elapsed);
        }

        if let Ok(mut guard) = self.last_rate_update.write() {
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
        let mut total_throughput = 0.0f64;

        for entry in self.breakers.iter() {
            let snap = entry.value().snapshot(entry.key());
            total_calls += snap.total_calls;
            total_successes += snap.successes;
            total_failures += snap.failures;
            total_timeouts += snap.timeouts;
            total_rejections += snap.rejections;
            total_throughput += snap.throughput;
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
            aggregate_failure_rate,
            aggregate_throughput: total_throughput,
        }
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        for entry in self.breakers.iter() {
            entry.value().successes.reset();
            entry.value().failures.reset();
            entry.value().timeouts.reset();
            entry.value().rejections.reset();
            entry.value().latency.reset();
        }
    }

    pub fn remove(&self, breaker_id: &str) {
        self.breakers.remove(breaker_id);
    }
}
