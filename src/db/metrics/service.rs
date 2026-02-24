//! Database metrics for Netflix-grade observability.
//!
//! Provides atomic counters and gauges for database pool and query metrics.
//! Integrates with the analytics module for centralized metrics collection.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Database metrics collector with lock-free atomic counters.
///
/// Tracks query counts, latencies, and circuit breaker state for
/// comprehensive database observability.
#[derive(Debug)]
pub struct DatabaseMetrics {
    /// Total queries started
    queries_started: AtomicU64,
    /// Successful queries
    queries_succeeded: AtomicU64,
    /// Failed queries
    queries_failed: AtomicU64,
    /// Queries rejected by circuit breaker
    queries_rejected: AtomicU64,
    /// Circuit breaker open events
    circuit_opens: AtomicU64,
    /// Total query latency in microseconds (for average calculation)
    total_latency_us: AtomicU64,
    /// Query start time for latency tracking
    last_query_start: std::sync::RwLock<Option<Instant>>,
}

impl Default for DatabaseMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl DatabaseMetrics {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            queries_started: AtomicU64::new(0),
            queries_succeeded: AtomicU64::new(0),
            queries_failed: AtomicU64::new(0),
            queries_rejected: AtomicU64::new(0),
            circuit_opens: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            last_query_start: std::sync::RwLock::new(None),
        }
    }

    /// Record query start.
    pub fn record_query_start(&self) {
        self.queries_started.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_query_start.write() {
            *guard = Some(Instant::now());
        }
    }

    /// Record successful query completion.
    pub fn record_query_success(&self) {
        self.queries_succeeded.fetch_add(1, Ordering::Relaxed);
        self.record_latency();
    }

    /// Record failed query.
    pub fn record_query_failure(&self) {
        self.queries_failed.fetch_add(1, Ordering::Relaxed);
        self.record_latency();
    }

    /// Record query rejection by circuit breaker.
    pub fn record_query_rejected(&self) {
        self.queries_rejected.fetch_add(1, Ordering::Relaxed);
    }

    /// Record circuit breaker open event.
    pub fn record_circuit_open(&self) {
        self.circuit_opens.fetch_add(1, Ordering::Relaxed);
    }

    /// Record latency from last query start.
    fn record_latency(&self) {
        if let Ok(guard) = self.last_query_start.read() {
            if let Some(start) = *guard {
                let latency_us = start.elapsed().as_micros() as u64;
                self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);
            }
        }
    }

    /// Get total queries started.
    pub fn queries_started(&self) -> u64 {
        self.queries_started.load(Ordering::Relaxed)
    }

    /// Get successful query count.
    pub fn queries_succeeded(&self) -> u64 {
        self.queries_succeeded.load(Ordering::Relaxed)
    }

    /// Get failed query count.
    pub fn queries_failed(&self) -> u64 {
        self.queries_failed.load(Ordering::Relaxed)
    }

    /// Get rejected query count.
    pub fn queries_rejected(&self) -> u64 {
        self.queries_rejected.load(Ordering::Relaxed)
    }

    /// Get circuit open count.
    pub fn circuit_opens(&self) -> u64 {
        self.circuit_opens.load(Ordering::Relaxed)
    }

    /// Get average latency in microseconds.
    pub fn average_latency_us(&self) -> u64 {
        let total = self.total_latency_us.load(Ordering::Relaxed);
        let completed = self.queries_succeeded() + self.queries_failed();
        if completed > 0 {
            total / completed
        } else {
            0
        }
    }

    /// Get success rate (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        let succeeded = self.queries_succeeded() as f64;
        let failed = self.queries_failed() as f64;
        let total = succeeded + failed;
        if total > 0.0 {
            succeeded / total
        } else {
            1.0
        }
    }

    /// Get a snapshot of all metrics.
    pub fn snapshot(&self) -> DatabaseMetricsSnapshot {
        DatabaseMetricsSnapshot {
            queries_started: self.queries_started(),
            queries_succeeded: self.queries_succeeded(),
            queries_failed: self.queries_failed(),
            queries_rejected: self.queries_rejected(),
            circuit_opens: self.circuit_opens(),
            average_latency_us: self.average_latency_us(),
            success_rate: self.success_rate(),
        }
    }

    /// Reset all metrics (for testing or periodic reset).
    pub fn reset(&self) {
        self.queries_started.store(0, Ordering::Relaxed);
        self.queries_succeeded.store(0, Ordering::Relaxed);
        self.queries_failed.store(0, Ordering::Relaxed);
        self.queries_rejected.store(0, Ordering::Relaxed);
        self.circuit_opens.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
    }
}

/// Immutable snapshot of database metrics.
#[derive(Debug, Clone)]
pub struct DatabaseMetricsSnapshot {
    pub queries_started: u64,
    pub queries_succeeded: u64,
    pub queries_failed: u64,
    pub queries_rejected: u64,
    pub circuit_opens: u64,
    pub average_latency_us: u64,
    pub success_rate: f64,
}

impl std::fmt::Display for DatabaseMetricsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DB[started={}, ok={}, fail={}, rejected={}, opens={}, lat={}μs, rate={:.2}%]",
            self.queries_started,
            self.queries_succeeded,
            self.queries_failed,
            self.queries_rejected,
            self.circuit_opens,
            self.average_latency_us,
            self.success_rate * 100.0
        )
    }
}
