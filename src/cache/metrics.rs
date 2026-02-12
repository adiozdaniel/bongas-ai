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
