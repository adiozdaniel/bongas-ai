//! Time-bucketed rolling window for failure rate tracking.
//!
//! Implements a ring buffer of time slices, each tracking success, failure,
//! and timeout counts. Old buckets are automatically expired and recycled.
//! This is the core data structure that makes the circuit breaker's decisions
//! based on recent history rather than all-time counters.
//!
//! Inspired by Netflix Hystrix's rolling number implementation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A single time bucket in the rolling window.
struct Bucket {
    successes: AtomicU64,
    failures: AtomicU64,
    timeouts: AtomicU64,
    rejections: AtomicU64,
    start_time: Instant,
}

impl Bucket {
    fn new(start_time: Instant) -> Self {
        Self {
            successes: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            rejections: AtomicU64::new(0),
            start_time,
        }
    }

    fn reset(&mut self, start_time: Instant) {
        self.successes.store(0, Ordering::SeqCst);
        self.failures.store(0, Ordering::SeqCst);
        self.timeouts.store(0, Ordering::SeqCst);
        self.rejections.store(0, Ordering::SeqCst);
        self.start_time = start_time;
    }
}

/// Snapshot of the rolling window's aggregate counters.
#[derive(Debug, Clone, Copy)]
pub struct WindowSnapshot {
    pub successes: u64,
    pub failures: u64,
    pub timeouts: u64,
    pub rejections: u64,
    pub total_calls: u64,
    pub failure_rate: f64,
}

/// Time-bucketed rolling window.
///
/// Divides `window_duration` into `bucket_count` equal slices. Each call
/// to `record_*` lands in the current bucket. Buckets older than
/// `window_duration` are expired on the next access.
pub struct RollingWindow {
    buckets: Vec<Bucket>,
    bucket_count: usize,
    bucket_duration: Duration,
    window_duration: Duration,
    current_index: usize,
    last_rotation: Instant,
}

impl RollingWindow {
    /// Creates a new rolling window.
    ///
    /// * `window_duration` — total time span the window covers.
    /// * `bucket_count` — number of time slices within the window.
    ///
    /// For example, `window_duration = 10s` and `bucket_count = 10` gives
    /// 1-second buckets with 10 seconds of history.
    pub fn new(window_duration: Duration, bucket_count: usize) -> Self {
        let bucket_count = bucket_count.max(1);
        let bucket_duration = window_duration / bucket_count as u32;
        let now = Instant::now();

        let buckets = (0..bucket_count)
            .map(|_| Bucket::new(now))
            .collect();

        Self {
            buckets,
            bucket_count,
            bucket_duration,
            window_duration,
            current_index: 0,
            last_rotation: now,
        }
    }

    /// Record a successful call.
    pub fn record_success(&mut self) {
        self.rotate_if_needed();
        self.buckets[self.current_index]
            .successes
            .fetch_add(1, Ordering::SeqCst);
    }

    /// Record a failed call.
    pub fn record_failure(&mut self) {
        self.rotate_if_needed();
        self.buckets[self.current_index]
            .failures
            .fetch_add(1, Ordering::SeqCst);
    }

    /// Record a timed-out call.
    pub fn record_timeout(&mut self) {
        self.rotate_if_needed();
        self.buckets[self.current_index]
            .timeouts
            .fetch_add(1, Ordering::SeqCst);
    }

    /// Record a rejected call (circuit was open).
    pub fn record_rejection(&mut self) {
        self.rotate_if_needed();
        self.buckets[self.current_index]
            .rejections
            .fetch_add(1, Ordering::SeqCst);
    }

    /// Take a snapshot of the current window state.
    ///
    /// Aggregates all non-expired buckets into a single summary.
    pub fn snapshot(&self) -> WindowSnapshot {
        let now = Instant::now();
        let mut successes = 0u64;
        let mut failures = 0u64;
        let mut timeouts = 0u64;
        let mut rejections = 0u64;

        for bucket in &self.buckets {
            if now.duration_since(bucket.start_time) <= self.window_duration {
                successes += bucket.successes.load(Ordering::SeqCst);
                failures += bucket.failures.load(Ordering::SeqCst);
                timeouts += bucket.timeouts.load(Ordering::SeqCst);
                rejections += bucket.rejections.load(Ordering::SeqCst);
            }
        }

        let total_calls = successes + failures + timeouts;
        let failure_rate = if total_calls > 0 {
            (failures + timeouts) as f64 / total_calls as f64
        } else {
            0.0
        };

        WindowSnapshot {
            successes,
            failures,
            timeouts,
            rejections,
            total_calls,
            failure_rate,
        }
    }

    /// Reset all buckets. Called when the circuit transitions to Closed.
    pub fn reset(&mut self) {
        let now = Instant::now();
        for bucket in &mut self.buckets {
            bucket.reset(now);
        }
        self.current_index = 0;
        self.last_rotation = now;
    }

    /// Advance to the next bucket if enough time has elapsed.
    fn rotate_if_needed(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_rotation);

        if elapsed >= self.bucket_duration {
            let rotations = (elapsed.as_nanos() / self.bucket_duration.as_nanos()) as usize;
            let rotations = rotations.min(self.bucket_count);

            for _ in 0..rotations {
                self.current_index = (self.current_index + 1) % self.bucket_count;
                self.buckets[self.current_index].reset(now);
            }

            self.last_rotation = now;
        }
    }
}
