//! HDR Histogram for latency percentile tracking.
//!
//! Provides O(1) recording and O(1) percentile queries with configurable
//! precision. Uses logarithmic bucketing to cover a wide range of latencies
//! (1μs to 1 hour) with bounded memory.

use std::sync::atomic::{AtomicU64, Ordering};

/// Number of buckets per power of 2 (precision).
const BUCKETS_PER_POWER: usize = 8;

/// Maximum value in microseconds (1 hour).
const MAX_VALUE_MICROS: u64 = 3_600_000_000;

/// Total number of buckets.
const BUCKET_COUNT: usize = 64 * BUCKETS_PER_POWER;

/// High Dynamic Range histogram for latency tracking.
///
/// Records latencies and provides percentile queries. Lock-free
/// via atomic operations on individual buckets.
pub struct HdrHistogram {
    buckets: Box<[AtomicU64; BUCKET_COUNT]>,
    count: AtomicU64,
    sum: AtomicU64,
    min: AtomicU64,
    max: AtomicU64,
}

impl HdrHistogram {
    pub fn new() -> Self {
        // Initialize array of AtomicU64
        let buckets: Box<[AtomicU64; BUCKET_COUNT]> =
            std::array::from_fn(|_| AtomicU64::new(0)).into();

        Self {
            buckets,
            count: AtomicU64::new(0),
            sum: AtomicU64::new(0),
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
        }
    }

    /// Record a latency value in microseconds.
    pub fn record(&self, value_micros: u64) {
        let value = value_micros.clamp(1, MAX_VALUE_MICROS);
        let bucket = self.value_to_bucket(value);

        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum.fetch_add(value, Ordering::Relaxed);

        // Update min
        let mut current_min = self.min.load(Ordering::Relaxed);
        while value < current_min {
            match self.min.compare_exchange_weak(
                current_min,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_min = v,
            }
        }

        // Update max
        let mut current_max = self.max.load(Ordering::Relaxed);
        while value > current_max {
            match self.max.compare_exchange_weak(
                current_max,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current_max = v,
            }
        }
    }

    /// Record a latency value from a Duration.
    pub fn record_duration(&self, duration: std::time::Duration) {
        self.record(duration.as_micros() as u64);
    }

    /// Get the value at a given percentile (0.0 to 100.0).
    pub fn percentile(&self, p: f64) -> u64 {
        let count = self.count.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }

        let target = ((p / 100.0) * count as f64).ceil() as u64;
        let mut accumulated = 0u64;

        for (i, bucket) in self.buckets.iter().enumerate() {
            accumulated += bucket.load(Ordering::Relaxed);
            if accumulated >= target {
                return self.bucket_to_value(i);
            }
        }

        self.max.load(Ordering::Relaxed)
    }

    /// Get common percentiles as (p50, p90, p95, p99).
    pub fn percentiles(&self) -> (u64, u64, u64, u64) {
        (
            self.percentile(50.0),
            self.percentile(90.0),
            self.percentile(95.0),
            self.percentile(99.0),
        )
    }

    /// Get the mean value in microseconds.
    pub fn mean(&self) -> f64 {
        let count = self.count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        self.sum.load(Ordering::Relaxed) as f64 / count as f64
    }

    /// Get the total count of recorded values.
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get the minimum recorded value.
    pub fn min(&self) -> u64 {
        let min = self.min.load(Ordering::Relaxed);
        if min == u64::MAX { 0 } else { min }
    }

    /// Get the maximum recorded value.
    pub fn max(&self) -> u64 {
        self.max.load(Ordering::Relaxed)
    }

    /// Reset the histogram.
    pub fn reset(&self) {
        for bucket in self.buckets.iter() {
            bucket.store(0, Ordering::Relaxed);
        }
        self.count.store(0, Ordering::Relaxed);
        self.sum.store(0, Ordering::Relaxed);
        self.min.store(u64::MAX, Ordering::Relaxed);
        self.max.store(0, Ordering::Relaxed);
    }

    /// Map a value to a bucket index using logarithmic scaling.
    fn value_to_bucket(&self, value: u64) -> usize {
        if value == 0 {
            return 0;
        }

        let log2 = 63 - value.leading_zeros() as usize;
        let base_bucket = log2 * BUCKETS_PER_POWER;

        // Sub-bucket within this power of 2
        let sub_bucket = if log2 == 0 {
            0
        } else {
            let mask = (1u64 << log2) - 1;
            let remainder = value & mask;
            ((remainder * BUCKETS_PER_POWER as u64) >> log2) as usize
        };

        (base_bucket + sub_bucket).min(BUCKET_COUNT - 1)
    }

    /// Map a bucket index back to its representative value.
    fn bucket_to_value(&self, bucket: usize) -> u64 {
        let power = bucket / BUCKETS_PER_POWER;
        let sub = bucket % BUCKETS_PER_POWER;

        if power == 0 {
            (sub + 1) as u64
        } else {
            let base = 1u64 << power;
            let step = base / BUCKETS_PER_POWER as u64;
            base + (sub as u64 * step)
        }
    }
}

impl Default for HdrHistogram {
    fn default() -> Self {
        Self::new()
    }
}
