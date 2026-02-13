//! Core metric types for the analytics module.
//!
//! Lock-free atomic primitives for high-throughput metrics collection.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Duration;

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

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

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

    pub fn set(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}

/// Throughput rate calculator.
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
            if let Ok(mut guard) = self.rate.write() {
                *guard = delta as f64 / secs;
            }
        }
    }

    pub fn get(&self) -> f64 {
        self.rate.read().map(|g| *g).unwrap_or(0.0)
    }
}

impl Default for Rate {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub failure_rate: f64,
    pub throughput: f64,
    pub latency_p50_ms: f64,
    pub latency_p90_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub latency_mean_ms: f64,
    pub concurrent_calls: i64,
    pub state_duration_secs: f64,
}

/// Point-in-time snapshot of all circuit breaker metrics.
#[derive(Debug, Clone)]
pub struct RegistrySnapshot {
    pub breakers: Vec<BreakerSnapshot>,
    pub total_calls: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_timeouts: u64,
    pub total_rejections: u64,
    pub aggregate_failure_rate: f64,
    pub aggregate_throughput: f64,
}
