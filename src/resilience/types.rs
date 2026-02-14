//! Core metric types for the analytics module.
//!
//! Lock-free atomic primitives for high-throughput metrics collection.
//!
//! # Netflix Resilience Features
//! - **Lock-free**: All primitives use atomic operations
//! - **Error Classification Tracking**: Counters by classification type
//! - **Degraded/Partial Failure Tracking**: Separate metrics for degraded responses

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{RwLock, PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

use crate::error::ErrorClassification;

// ─── Counter ────────────────────────────────────────────────────────────────

/// Lock-free counter for monotonically increasing values.
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Gauge ──────────────────────────────────────────────────────────────────

/// Lock-free gauge for values that can increase or decrease.
pub struct Gauge {
    value: AtomicI64,
}

impl Gauge {
    pub fn new() -> Self {
        Self {
            value: AtomicI64::new(0),
        }
    }

    #[inline]
    pub fn set(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn decrement(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Rate ───────────────────────────────────────────────────────────────────

/// Throughput rate calculator.
///
/// Calculates events per second based on count deltas over time.
pub struct Rate {
    last_count: AtomicU64,
    rate: RwLock<f64>,
}

impl Rate {
    pub fn new() -> Self {
        Self {
            last_count: AtomicU64::new(0),
            rate: RwLock::new(0.0),
        }
    }

    pub fn update(&self, current_count: u64, elapsed: Duration) {
        let last = self.last_count.swap(current_count, Ordering::Relaxed);
        let delta = current_count.saturating_sub(last);
        let secs = elapsed.as_secs_f64();

        if secs > 0.0 {
            if let Ok(mut guard) = self.write_rate() {
                *guard = delta as f64 / secs;
            }
        }
    }

    #[inline]
    pub fn get(&self) -> f64 {
        self.read_rate().map(|g| *g).unwrap_or(0.0)
    }

    #[inline]
    pub fn reset(&self) {
        self.last_count.store(0, Ordering::Relaxed);
        if let Ok(mut guard) = self.write_rate() {
            *guard = 0.0;
        }
    }

    // Lock poison recovery
    #[inline]
    fn read_rate(&self) -> Result<RwLockReadGuard<'_, f64>, ()> {
        self.rate.read().map_err(|_| ()).or_else(|_| {
            Ok(self.rate.read().unwrap_or_else(PoisonError::into_inner))
        })
    }

    #[inline]
    fn write_rate(&self) -> Result<RwLockWriteGuard<'_, f64>, ()> {
        self.rate.write().map_err(|_| ()).or_else(|_| {
            Ok(self.rate.write().unwrap_or_else(PoisonError::into_inner))
        })
    }
}

impl Default for Rate {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Error Classification Counters ──────────────────────────────────────────

/// Counters for each error classification type.
///
/// Enables detailed breakdown of failures by category for resilience analysis.
pub struct ClassificationCounters {
    transient: Counter,
    permanent: Counter,
    timeout: Counter,
    overload: Counter,
    degraded: Counter,
    partial_failure: Counter,
}

impl ClassificationCounters {
    pub fn new() -> Self {
        Self {
            transient: Counter::new(),
            permanent: Counter::new(),
            timeout: Counter::new(),
            overload: Counter::new(),
            degraded: Counter::new(),
            partial_failure: Counter::new(),
        }
    }

    /// Increment the counter for the given classification.
    #[inline]
    pub fn record(&self, classification: ErrorClassification) {
        match classification {
            ErrorClassification::Transient => self.transient.increment(),
            ErrorClassification::Permanent => self.permanent.increment(),
            ErrorClassification::Timeout => self.timeout.increment(),
            ErrorClassification::Overload => self.overload.increment(),
            ErrorClassification::Degraded => self.degraded.increment(),
            ErrorClassification::PartialFailure => self.partial_failure.increment(),
        }
    }

    #[inline]
    pub fn transient(&self) -> u64 {
        self.transient.get()
    }

    #[inline]
    pub fn permanent(&self) -> u64 {
        self.permanent.get()
    }

    #[inline]
    pub fn timeout(&self) -> u64 {
        self.timeout.get()
    }

    #[inline]
    pub fn overload(&self) -> u64 {
        self.overload.get()
    }

    #[inline]
    pub fn degraded(&self) -> u64 {
        self.degraded.get()
    }

    #[inline]
    pub fn partial_failure(&self) -> u64 {
        self.partial_failure.get()
    }

    /// Total errors that should trip the circuit breaker.
    #[inline]
    pub fn trippable(&self) -> u64 {
        self.transient.get() + self.timeout.get() + self.overload.get()
    }

    /// Total errors that are retriable.
    #[inline]
    pub fn retriable(&self) -> u64 {
        self.transient.get() + self.timeout.get() + self.partial_failure.get()
    }

    /// Reset all counters.
    pub fn reset(&self) {
        self.transient.reset();
        self.permanent.reset();
        self.timeout.reset();
        self.overload.reset();
        self.degraded.reset();
        self.partial_failure.reset();
    }

    /// Take a snapshot of all counters.
    pub fn snapshot(&self) -> ClassificationSnapshot {
        ClassificationSnapshot {
            transient: self.transient.get(),
            permanent: self.permanent.get(),
            timeout: self.timeout.get(),
            overload: self.overload.get(),
            degraded: self.degraded.get(),
            partial_failure: self.partial_failure.get(),
        }
    }
}

impl Default for ClassificationCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of error classification counters.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClassificationSnapshot {
    pub transient: u64,
    pub permanent: u64,
    pub timeout: u64,
    pub overload: u64,
    pub degraded: u64,
    pub partial_failure: u64,
}

impl ClassificationSnapshot {
    /// Total errors that should trip the circuit breaker.
    #[inline]
    pub fn trippable(&self) -> u64 {
        self.transient + self.timeout + self.overload
    }

    /// Total errors that are retriable.
    #[inline]
    pub fn retriable(&self) -> u64 {
        self.transient + self.timeout + self.partial_failure
    }

    /// Total of all errors.
    #[inline]
    pub fn total(&self) -> u64 {
        self.transient
            + self.permanent
            + self.timeout
            + self.overload
            + self.degraded
            + self.partial_failure
    }
}

// ─── Breaker Snapshot ───────────────────────────────────────────────────────

/// Point-in-time snapshot of a single circuit breaker's metrics.
#[derive(Debug, Clone)]
pub struct BreakerSnapshot {
    pub breaker_id: String,
    pub state: String,
    pub total_calls: u64,
    pub successes: u64,
    pub failures: u64,
    pub timeouts: u64,
    pub rejections: u64,
    pub slow_calls: u64,
    pub degraded_calls: u64,
    pub failure_rate: f64,
    pub throughput: f64,
    pub latency_p50_ms: f64,
    pub latency_p90_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub latency_mean_ms: f64,
    pub concurrent_calls: i64,
    pub state_duration_secs: f64,
    /// Error breakdown by classification.
    pub classifications: ClassificationSnapshot,
}

// ─── Registry Snapshot ──────────────────────────────────────────────────────

/// Point-in-time snapshot of all circuit breaker metrics.
#[derive(Debug, Clone)]
pub struct RegistrySnapshot {
    pub breakers: Vec<BreakerSnapshot>,
    pub total_calls: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_timeouts: u64,
    pub total_rejections: u64,
    pub total_slow_calls: u64,
    pub total_degraded: u64,
    pub aggregate_failure_rate: f64,
    pub aggregate_throughput: f64,
    
    /// Aggregate error breakdown by classification.
    pub aggregate_classifications: ClassificationSnapshot,
}
