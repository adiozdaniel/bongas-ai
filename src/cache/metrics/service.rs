  //! Cache metrics for observability.

  use std::sync::atomic::{AtomicU64, Ordering};
  use std::sync::Arc;

  /// Cache metrics collector.
  #[derive(Debug, Clone)]
  pub struct CacheMetrics {
      l1_hits: Arc<AtomicU64>,
      l1_misses: Arc<AtomicU64>,
      l2_hits: Arc<AtomicU64>,
      l2_misses: Arc<AtomicU64>,
      evictions: Arc<AtomicU64>,
      invalidations: Arc<AtomicU64>,
      errors: Arc<AtomicU64>,
  }

  impl CacheMetrics {
      pub fn new() -> Self {
          Self {
              l1_hits: Arc::new(AtomicU64::new(0)),
              l1_misses: Arc::new(AtomicU64::new(0)),
              l2_hits: Arc::new(AtomicU64::new(0)),
              l2_misses: Arc::new(AtomicU64::new(0)),
              evictions: Arc::new(AtomicU64::new(0)),
              invalidations: Arc::new(AtomicU64::new(0)),
              errors: Arc::new(AtomicU64::new(0)),
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

      pub fn record_eviction(&self) {
          self.evictions.fetch_add(1, Ordering::Relaxed);
      }

      pub fn record_invalidation(&self) {
          self.invalidations.fetch_add(1, Ordering::Relaxed);
      }

      pub fn record_error(&self) {
          self.errors.fetch_add(1, Ordering::Relaxed);
      }

      pub fn snapshot(&self) -> CacheMetricsSnapshot {
          CacheMetricsSnapshot {
              l1_hits: self.l1_hits.load(Ordering::Relaxed),
              l1_misses: self.l1_misses.load(Ordering::Relaxed),
              l2_hits: self.l2_hits.load(Ordering::Relaxed),
              l2_misses: self.l2_misses.load(Ordering::Relaxed),
              evictions: self.evictions.load(Ordering::Relaxed),
              invalidations: self.invalidations.load(Ordering::Relaxed),
              errors: self.errors.load(Ordering::Relaxed),
          }
      }
  }

  impl Default for CacheMetrics {
      fn default() -> Self {
          Self::new()
      }
  }

  #[derive(Debug, Clone)]
  pub struct CacheMetricsSnapshot {
      pub l1_hits: u64,
      pub l1_misses: u64,
      pub l2_hits: u64,
      pub l2_misses: u64,
      pub evictions: u64,
      pub invalidations: u64,
      pub errors: u64,
  }

  impl CacheMetricsSnapshot {
      pub fn l1_hit_rate(&self) -> f64 {
          let total = self.l1_hits + self.l1_misses;
          if total == 0 {
              0.0
          } else {
              self.l1_hits as f64 / total as f64
          }
      }

      pub fn l2_hit_rate(&self) -> f64 {
          let total = self.l2_hits + self.l2_misses;
          if total == 0 {
              0.0
          } else {
              self.l2_hits as f64 / total as f64
          }
      }

      pub fn overall_hit_rate(&self) -> f64 {
          let total_hits = self.l1_hits + self.l2_hits;
          let total_requests = total_hits + self.l2_misses;
          if total_requests == 0 {
              0.0
          } else {
              total_hits as f64 / total_requests as f64
          }
      }
  }

