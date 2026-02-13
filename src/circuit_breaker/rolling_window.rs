//! Lock-free time-bucketed rolling window for failure rate tracking.
//!
//! Implements a ring buffer of time slices using pure atomic operations.
//! No mutex required - all operations are lock-free for maximum throughput.
//! Tracks successes, failures, timeouts, slow calls, and rejections.
//!
//! Inspired by Netflix Hystrix's rolling number implementation.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{RwLock, PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};

/// Counters for a single time bucket.
/// All counters are atomic for lock-free concurrent updates.
#[derive(Debug)]
struct BucketCounters {
    successes: AtomicU64,
    failures: AtomicU64,
    timeouts: AtomicU64,
    slow_calls: AtomicU64,
    rejections: AtomicU64,
    /// Total latency in microseconds for percentile calculations.
    total_latency_us: AtomicU64,
    /// Number of calls contributing to total_latency_us.
    latency_count: AtomicU64,
}

impl BucketCounters {
    fn new() -> Self {
        Self {
            successes: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            slow_calls: AtomicU64::new(0),
            rejections: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
        }
    }

    fn reset(&self) {
        self.successes.store(0, Ordering::Release);
        self.failures.store(0, Ordering::Release);
        self.timeouts.store(0, Ordering::Release);
        self.slow_calls.store(0, Ordering::Release);
        self.rejections.store(0, Ordering::Release);
        self.total_latency_us.store(0, Ordering::Release);
        self.latency_count.store(0, Ordering::Release);
    }
}

/// A single time bucket in the rolling window.
struct Bucket {
    counters: BucketCounters,
    /// Epoch number to detect stale buckets without locks.
    /// Incremented each time this bucket is recycled.
    epoch: AtomicU64,
}

impl Bucket {
    fn new() -> Self {
        Self {
            counters: BucketCounters::new(),
            epoch: AtomicU64::new(0),
        }
    }

    /// Reset counters and increment epoch.
    fn recycle(&self) {
        self.counters.reset();
        self.epoch.fetch_add(1, Ordering::AcqRel);
    }
}

/// Snapshot of the rolling window's aggregate counters.
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowSnapshot {
    pub successes: u64,
    pub failures: u64,
    pub timeouts: u64,
    pub slow_calls: u64,
    pub rejections: u64,
    /// Total calls that completed (success + failure + timeout).
    pub total_calls: u64,
    /// Failure rate: (failures + timeouts) / total_calls.
    pub failure_rate: f64,
    /// Slow call rate: slow_calls / total_calls.
    pub slow_call_rate: f64,
    /// Average latency in microseconds (0 if no calls).
    pub avg_latency_us: u64,
}

impl WindowSnapshot {
    /// Returns true if the window has enough data for evaluation.
    #[inline]
    pub fn has_minimum_calls(&self, minimum: u64) -> bool {
        self.total_calls >= minimum
    }

    /// Returns true if failure rate exceeds the threshold.
    #[inline]
    pub fn exceeds_failure_threshold(&self, threshold: f64) -> bool {
        self.failure_rate >= threshold
    }

    /// Returns true if slow call rate exceeds the threshold.
    #[inline]
    pub fn exceeds_slow_call_threshold(&self, threshold: f64) -> bool {
        self.slow_call_rate >= threshold
    }
}

/// Time tracking for bucket rotation.
struct TimeTracker {
    /// When the window was created or last fully reset.
    base_time: Instant,
    /// Duration of each bucket.
    bucket_duration: Duration,
}

impl TimeTracker {
    fn new(bucket_duration: Duration, _window_duration: Duration) -> Self {
        Self {
            base_time: Instant::now(),
            bucket_duration,
        }
    }

    /// Calculate which bucket index a given instant maps to.
    fn bucket_index_for(&self, instant: Instant, bucket_count: usize) -> usize {
        let elapsed = instant.saturating_duration_since(self.base_time);
        let bucket_nanos = self.bucket_duration.as_nanos() as u64;
        if bucket_nanos == 0 {
            return 0;
        }
        let bucket_number = elapsed.as_nanos() as u64 / bucket_nanos;
        (bucket_number as usize) % bucket_count
    }

    /// Calculate the epoch for a given instant.
    fn epoch_for(&self, instant: Instant, bucket_count: usize) -> u64 {
        let elapsed = instant.saturating_duration_since(self.base_time);
        let bucket_nanos = self.bucket_duration.as_nanos() as u64;
        if bucket_nanos == 0 {
            return 0;
        }
        let bucket_number = elapsed.as_nanos() as u64 / bucket_nanos;
        bucket_number / bucket_count as u64
    }

    /// Check if a bucket's epoch is current (not stale).
    fn is_bucket_current(&self, bucket_epoch: u64, now: Instant, bucket_count: usize) -> bool {
        let current_epoch = self.epoch_for(now, bucket_count);
        // Bucket is current if it's from the current epoch or the previous one
        // (to handle boundary conditions)
        bucket_epoch >= current_epoch.saturating_sub(1)
    }

    fn reset(&mut self) {
        self.base_time = Instant::now();
    }
}

/// Lock-free time-bucketed rolling window.
///
/// All recording operations are lock-free using atomic counters.
/// Only `snapshot()` and `reset()` use a lightweight read-write lock
/// to ensure consistency when reading across all buckets.
///
/// # Thread Safety
/// - `record_*` methods: Lock-free, can be called from any thread.
/// - `snapshot()`: Takes read lock, concurrent with other reads.
/// - `reset()`: Takes write lock, exclusive access.
pub struct RollingWindow {
    buckets: Vec<Bucket>,
    bucket_count: usize,
    /// Current bucket index (atomic for lock-free updates).
    current_index: AtomicUsize,
    /// Time tracking (protected by RwLock for reset).
    time_tracker: RwLock<TimeTracker>,
}

// RollingWindow is automatically Send + Sync because all fields are:
// - Vec<Bucket>: Bucket contains only atomics (Send + Sync)
// - usize: Send + Sync
// - AtomicUsize: Send + Sync
// - RwLock<TimeTracker>: Send + Sync

impl RollingWindow {
    /// Creates a new rolling window.
    ///
    /// * `window_duration` — total time span the window covers.
    /// * `bucket_count` — number of time slices within the window.
    ///
    /// # Panics
    /// Panics if `bucket_count` is 0 or `window_duration` is zero.
    pub fn new(window_duration: Duration, bucket_count: usize) -> Self {
        assert!(bucket_count > 0, "bucket_count must be at least 1");
        assert!(!window_duration.is_zero(), "window_duration must be positive");

        let bucket_duration = window_duration / bucket_count as u32;
        assert!(
            !bucket_duration.is_zero(),
            "window_duration too short for bucket_count"
        );

        let buckets = (0..bucket_count).map(|_| Bucket::new()).collect();
        let time_tracker = TimeTracker::new(bucket_duration, window_duration);

        Self {
            buckets,
            bucket_count,
            current_index: AtomicUsize::new(0),
            time_tracker: RwLock::new(time_tracker),
        }
    }

    /// Record a successful call with its latency.
    #[inline]
    pub fn record_success(&self, latency: Duration) {
        let bucket = self.current_bucket();
        bucket.counters.successes.fetch_add(1, Ordering::Relaxed);
        self.record_latency(bucket, latency);
    }

    /// Record a failed call with its latency.
    #[inline]
    pub fn record_failure(&self, latency: Duration) {
        let bucket = self.current_bucket();
        bucket.counters.failures.fetch_add(1, Ordering::Relaxed);
        self.record_latency(bucket, latency);
    }

    /// Record a timed-out call.
    #[inline]
    pub fn record_timeout(&self, latency: Duration) {
        let bucket = self.current_bucket();
        bucket.counters.timeouts.fetch_add(1, Ordering::Relaxed);
        self.record_latency(bucket, latency);
    }

    /// Record a slow call (latency exceeded threshold).
    #[inline]
    pub fn record_slow_call(&self) {
        let bucket = self.current_bucket();
        bucket.counters.slow_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rejected call (circuit was open).
    #[inline]
    pub fn record_rejection(&self) {
        let bucket = self.current_bucket();
        bucket.counters.rejections.fetch_add(1, Ordering::Relaxed);
    }

    /// Take a snapshot of the current window state.
    ///
    /// Aggregates all non-expired buckets into a single summary.
    /// This operation takes a read lock to ensure consistency.
    pub fn snapshot(&self) -> WindowSnapshot {
        let now = Instant::now();
        let time_tracker = self.read_time_tracker();

        let mut successes = 0u64;
        let mut failures = 0u64;
        let mut timeouts = 0u64;
        let mut slow_calls = 0u64;
        let mut rejections = 0u64;
        let mut total_latency_us = 0u64;
        let mut latency_count = 0u64;

        for bucket in &self.buckets {
            let epoch = bucket.epoch.load(Ordering::Acquire);

            // Only include bucket if it's from a recent epoch
            if time_tracker.is_bucket_current(epoch, now, self.bucket_count) {
                successes += bucket.counters.successes.load(Ordering::Acquire);
                failures += bucket.counters.failures.load(Ordering::Acquire);
                timeouts += bucket.counters.timeouts.load(Ordering::Acquire);
                slow_calls += bucket.counters.slow_calls.load(Ordering::Acquire);
                rejections += bucket.counters.rejections.load(Ordering::Acquire);
                total_latency_us += bucket.counters.total_latency_us.load(Ordering::Acquire);
                latency_count += bucket.counters.latency_count.load(Ordering::Acquire);
            }
        }

        let total_calls = successes + failures + timeouts;

        let failure_rate = if total_calls > 0 {
            (failures + timeouts) as f64 / total_calls as f64
        } else {
            0.0
        };

        let slow_call_rate = if total_calls > 0 {
            slow_calls as f64 / total_calls as f64
        } else {
            0.0
        };

        let avg_latency_us = if latency_count > 0 {
            total_latency_us / latency_count
        } else {
            0
        };

        WindowSnapshot {
            successes,
            failures,
            timeouts,
            slow_calls,
            rejections,
            total_calls,
            failure_rate,
            slow_call_rate,
            avg_latency_us,
        }
    }

    /// Reset all buckets. Called when the circuit transitions to Closed.
    ///
    /// This operation takes a write lock for exclusive access.
    pub fn reset(&self) {
        let mut time_tracker = self.write_time_tracker();
        time_tracker.reset();

        for bucket in &self.buckets {
            bucket.recycle();
        }

        self.current_index.store(0, Ordering::Release);
    }

    // ─── Internal ───────────────────────────────────────────────────────

    /// Get the current bucket, rotating if necessary.
    #[inline]
    fn current_bucket(&self) -> &Bucket {
        let now = Instant::now();
        let time_tracker = self.read_time_tracker();

        let target_index = time_tracker.bucket_index_for(now, self.bucket_count);
        let target_epoch = time_tracker.epoch_for(now, self.bucket_count);

        // Try to update current index (benign race - multiple threads may update)
        let _ = self.current_index.compare_exchange(
            self.current_index.load(Ordering::Relaxed),
            target_index,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );

        let bucket = &self.buckets[target_index];

        // Check if bucket needs recycling (stale epoch)
        let bucket_epoch = bucket.epoch.load(Ordering::Acquire);
        if bucket_epoch < target_epoch {
            // Try to recycle - CAS to prevent double recycling
            if bucket
                .epoch
                .compare_exchange(bucket_epoch, target_epoch, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                bucket.counters.reset();
            }
        }

        bucket
    }

    /// Record latency in the bucket.
    #[inline]
    fn record_latency(&self, bucket: &Bucket, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        bucket
            .counters
            .total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
        bucket.counters.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Acquire read lock, recovering from poison if necessary.
    #[inline]
    fn read_time_tracker(&self) -> RwLockReadGuard<'_, TimeTracker> {
        self.time_tracker.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Acquire write lock, recovering from poison if necessary.
    #[inline]
    fn write_time_tracker(&self) -> RwLockWriteGuard<'_, TimeTracker> {
        self.time_tracker.write().unwrap_or_else(PoisonError::into_inner)
    }
}
