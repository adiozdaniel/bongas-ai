use std::sync::atomic::{AtomicU64, Ordering};

pub struct CacheMetrics {
    l1_hits: AtomicU64,
    l1_misses: AtomicU64,
    l2_hits: AtomicU64,
    l2_misses: AtomicU64,
    invalidations: AtomicU64,
    warmings: AtomicU64,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self {
            l1_hits: AtomicU64::new(0),
            l1_misses: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
            l2_misses: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
            warmings: AtomicU64::new(0),
        }
    }

    pub fn record_l1_hit(&self) {
        self.l1_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l1_miss(&self) {
        self.l1_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l2_hit(&self) {
        self.l2_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_l2_miss(&self) {
        self.l2_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_warming(&self) {
        self.warmings.fetch_add(1, Ordering::Relaxed);
    }

    /// Overall cache hit rate (L1 + L2 hits / total requests)
    pub fn get_hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits.load(Ordering::Relaxed)
            + self.l2_hits.load(Ordering::Relaxed);
        let total_misses = self.l1_misses.load(Ordering::Relaxed)
            + self.l2_misses.load(Ordering::Relaxed);

        if total_hits + total_misses == 0 {
            return 0.0;
        }

        total_hits as f64 / (total_hits + total_misses) as f64
    }

    /// L1-only hit rate
    pub fn get_l1_hit_rate(&self) -> f64 {
        let hits = self.l1_hits.load(Ordering::Relaxed);
        let misses = self.l1_misses.load(Ordering::Relaxed);

        if hits + misses == 0 {
            return 0.0;
        }

        hits as f64 / (hits + misses) as f64
    }

    /// L2-only hit rate (of L1 misses that hit L2)
    pub fn get_l2_hit_rate(&self) -> f64 {
        let hits = self.l2_hits.load(Ordering::Relaxed);
        let misses = self.l2_misses.load(Ordering::Relaxed);

        if hits + misses == 0 {
            return 0.0;
        }

        hits as f64 / (hits + misses) as f64
    }

    pub fn get_stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            l1_hits: self.l1_hits.load(Ordering::Relaxed),
            l1_misses: self.l1_misses.load(Ordering::Relaxed),
            l2_hits: self.l2_hits.load(Ordering::Relaxed),
            l2_misses: self.l2_misses.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            warmings: self.warmings.load(Ordering::Relaxed),
            overall_hit_rate: self.get_hit_rate(),
            l1_hit_rate: self.get_l1_hit_rate(),
            l2_hit_rate: self.get_l2_hit_rate(),
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.l1_hits.store(0, Ordering::Relaxed);
        self.l1_misses.store(0, Ordering::Relaxed);
        self.l2_hits.store(0, Ordering::Relaxed);
        self.l2_misses.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
        self.warmings.store(0, Ordering::Relaxed);
    }
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStatsSnapshot {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub invalidations: u64,
    pub warmings: u64,
    pub overall_hit_rate: f64,
    pub l1_hit_rate: f64,
    pub l2_hit_rate: f64,
}
