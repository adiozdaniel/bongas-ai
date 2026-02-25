//! Unified item feature service for pipeline stages.

use std::sync::Arc;
use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;

/// Resilient item feature service used by all pipeline stages.
///
/// Every query goes through `ResilientPool` → circuit breaker + bulkhead + timeout.
pub struct ItemFeatureService {
    pub(super) pool: Arc<ResilientPool>,
    pub(super) metrics: Arc<ResilienceMetricsCollector>,
}

impl ItemFeatureService {
    pub fn new(pool: Arc<ResilientPool>, metrics: Arc<ResilienceMetricsCollector>) -> Self {
        Self { pool, metrics }
    }

    pub fn pool(&self) -> &Arc<ResilientPool> {
        &self.pool
    }
}
