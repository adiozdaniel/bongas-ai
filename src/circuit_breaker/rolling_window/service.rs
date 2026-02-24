//! Lock-free time-bucketed rolling window implementation.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{RwLock, PoisonError, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Duration, Instant};
use super::models::WindowSnapshot;

/// Counters for a single time bucket.
#[derive(Debug)]
pub(crate) struct BucketCounters {
    pub(crate) successes: AtomicU64,
    pub(crate) failures: AtomicU64,
    pub(crate) timeouts: AtomicU64,
    pub(crate) slow_calls: AtomicU64,
    pub(crate) rejections: AtomicU64,
    pub(crate) total_latency_us: AtomicU64,
    pub(crate) latency_count: AtomicU64,
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
pub(crate) struct Bucket {
    pub(crate) counters: BucketCounters,
    pub(crate) epoch: AtomicU64,
}

impl Bucket {
    fn new() -> Self {
        Self {
            counters: BucketCounters::new(),
            epoch: AtomicU64::new(0),
        }
    }

    fn recycle(&self) {
        self.counters.reset();
        self.epoch.fetch_add(1, Ordering::AcqRel);
    }
}

/// Time tracking for bucket rotation.
pub(crate) struct TimeTracker {
    base_time: Instant,
    bucket_duration: Duration,
}

impl TimeTracker {
    fn new(bucket_duration: Duration) -> Self {
        Self {
            base_time: Instant::now(),
            bucket_duration,
        }
    }

    fn bucket_index_for(&self, instant: Instant, bucket_count: usize) -> usize {
        let elapsed = instant.saturating_duration_since(self.base_time);
        let bucket_nanos = self.bucket_duration.as_nanos() as u64;
        if bucket_nanos == 0 {
            return 0;
        }
        let bucket_number = elapsed.as_nanos() as u64 / bucket_nanos;
        (bucket_number as usize) % bucket_count
    }

    fn epoch_for(&self, instant: Instant, bucket_count: usize) -> u64 {
        let elapsed = instant.saturating_duration_since(self.base_time);
        let bucket_nanos = self.bucket_duration.as_nanos() as u64;
        if bucket_nanos == 0 {
            return 0;
        }
        let bucket_number = elapsed.as_nanos() as u64 / bucket_nanos;
        bucket_number / bucket_count as u64
    }

    fn is_bucket_current(&self, bucket_epoch: u64, now: Instant, bucket_count: usize) -> bool {
        let current_epoch = self.epoch_for(now, bucket_count);
        bucket_epoch >= current_epoch.saturating_sub(1)
    }

    fn reset(&mut self) {
        self.base_time = Instant::now();
    }
}

/// Lock-free time-bucketed rolling window.
pub struct RollingWindow {
    buckets: Vec<Bucket>,
    bucket_count: usize,
    current_index: AtomicUsize,
    time_tracker: RwLock<TimeTracker>,
}

impl RollingWindow {
    pub fn new(window_duration: Duration, bucket_count: usize) -> Self {
        assert!(bucket_count > 0, "bucket_count must be at least 1");
        assert!(!window_duration.is_zero(), "window_duration must be positive");

        let bucket_duration = window_duration / bucket_count as u32;
        assert!(
            !bucket_duration.is_zero(),
            "window_duration too short for bucket_count"
        );

        let buckets = (0..bucket_count).map(|_| Bucket::new()).collect();
        let time_tracker = TimeTracker::new(bucket_duration);

        Self {
            buckets,
            bucket_count,
            current_index: AtomicUsize::new(0),
            time_tracker: RwLock::new(time_tracker),
        }
    }

    #[inline]
    pub fn record_success(&self, latency: Duration) {
        let bucket = self.current_bucket();
        bucket.counters.successes.fetch_add(1, Ordering::Relaxed);
        self.record_latency(bucket, latency);
    }

    #[inline]
    pub fn record_failure(&self, latency: Duration) {
        let bucket = self.current_bucket();
        bucket.counters.failures.fetch_add(1, Ordering::Relaxed);
        self.record_latency(bucket, latency);
    }

    #[inline]
    pub fn record_timeout(&self, latency: Duration) {
        let bucket = self.current_bucket();
        bucket.counters.timeouts.fetch_add(1, Ordering::Relaxed);
        self.record_latency(bucket, latency);
    }

    #[inline]
    pub fn record_slow_call(&self) {
        let bucket = self.current_bucket();
        bucket.counters.slow_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_rejection(&self) {
        let bucket = self.current_bucket();
        bucket.counters.rejections.fetch_add(1, Ordering::Relaxed);
    }

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

    pub fn reset(&self) {
        let mut time_tracker = self.write_time_tracker();
        time_tracker.reset();

        for bucket in &self.buckets {
            bucket.recycle();
        }

        self.current_index.store(0, Ordering::Release);
    }

    #[inline]
    fn current_bucket(&self) -> &Bucket {
        let now = Instant::now();
        let time_tracker = self.read_time_tracker();

        let target_index = time_tracker.bucket_index_for(now, self.bucket_count);
        let target_epoch = time_tracker.epoch_for(now, self.bucket_count);

        let _ = self.current_index.compare_exchange(
            self.current_index.load(Ordering::Relaxed),
            target_index,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );

        let bucket = &self.buckets[target_index];

        let bucket_epoch = bucket.epoch.load(Ordering::Acquire);
        if bucket_epoch < target_epoch {
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

    #[inline]
    fn record_latency(&self, bucket: &Bucket, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        bucket
            .counters
            .total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
        bucket.counters.latency_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    fn read_time_tracker(&self) -> RwLockReadGuard<'_, TimeTracker> {
        self.time_tracker.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn write_time_tracker(&self) -> RwLockWriteGuard<'_, TimeTracker> {
        self.time_tracker.write().unwrap_or_else(PoisonError::into_inner)
    }
}
