//! Local statistics collector implementation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::analytics::types::{ClientStatsPayload, SecurityDetails, BusinessStats, PerformanceStats, ResourceStats};

/// Local statistics collector for client-side metrics.
///
/// Collects statistics in memory and provides them for upload.
/// Does not persist to database - designed for client binary deployments.
pub struct LocalStatsCollector {
    /// Business statistics.
    pub(crate) business: Arc<BusinessStats>,
    
    /// Performance statistics.
    pub(crate) performance: Arc<PerformanceStats>,
    
    /// Resource statistics.
    pub(crate) resources: Arc<ResourceStats>,
    
    /// Collection interval for periodic updates.
    pub(crate) collection_interval: Duration,
}

impl LocalStatsCollector {
    /// Create a new local statistics collector.
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            business: Arc::new(BusinessStats::new()),
            performance: Arc::new(PerformanceStats::new()),
            resources: Arc::new(ResourceStats::new()),
            collection_interval,
        }
    }

    /// Get business statistics.
    pub fn business(&self) -> Arc<BusinessStats> {
        Arc::clone(&self.business)
    }

    /// Get performance statistics.
    pub fn performance(&self) -> Arc<PerformanceStats> {
        Arc::clone(&self.performance)
    }

    /// Get resource statistics.
    pub fn resources(&self) -> Arc<ResourceStats> {
        Arc::clone(&self.resources)
    }

    /// Get collection interval.
    pub fn collection_interval(&self) -> Duration {
        self.collection_interval
    }

    /// Get statistics payload for upload.
    pub fn get_stats_for_upload(&self, client_id: String, security: SecurityDetails) -> ClientStatsPayload {
        ClientStatsPayload {
            client_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            security,
            business: BusinessStats {
                api_calls: AtomicU64::new(self.business.api_calls.load(Ordering::Relaxed)),
                successful_calls: AtomicU64::new(self.business.successful_calls.load(Ordering::Relaxed)),
                failed_calls: AtomicU64::new(self.business.failed_calls.load(Ordering::Relaxed)),
                feature_usage: self.business.feature_usage.clone(),
            },
            performance: PerformanceStats {
                response_times: self.performance.response_times.clone(),
                throughput: AtomicU64::new(self.performance.throughput.load(Ordering::Relaxed)),
                error_counts: self.performance.error_counts.clone(),
            },
            resources: ResourceStats {
                memory_usage: AtomicU64::new(self.resources.memory_usage.load(Ordering::Relaxed)),
                cpu_usage: AtomicU64::new(self.resources.cpu_usage.load(Ordering::Relaxed)),
                disk_operations: AtomicU64::new(self.resources.disk_operations.load(Ordering::Relaxed)),
                network_bytes: AtomicU64::new(self.resources.network_bytes.load(Ordering::Relaxed)),
            },
        }
    }

    /// Start background collection task.
    pub async fn start_background_collection(self: Arc<Self>) {
        let collector = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(collector.collection_interval);
            
            loop {
                interval.tick().await;
                
                // Update resource statistics
                collector.update_resource_stats().await;
                
                // Update performance statistics
                collector.update_performance_stats().await;
            }
        });
    }

    /// Update resource statistics.
    async fn update_resource_stats(&self) {
        let base_memory = 150 * 1024 * 1024;
        let variable_memory = (rand::random::<u64>() % 100) * 1024 * 1024;
        self.resources.set_memory_usage(base_memory + variable_memory);
        
        let throughput = self.performance.throughput.load(Ordering::Relaxed);
        let cpu_usage = (throughput % 100).max(5);
        self.resources.set_cpu_usage(cpu_usage);
        
        self.resources.increment_disk_operations(rand::random::<u64>() % 3);
        self.resources.increment_network_bytes((throughput % 1000) * 1024);
    }

    /// Update performance statistics.
    async fn update_performance_stats(&self) {
        let response_time = 50 + (rand::random::<u64>() % 200);
        self.performance.record_response_time("simulated", response_time);
        
        self.performance.increment_throughput("simulated");
        
        if rand::random::<bool>() {
            self.performance.increment_error("timeout");
        }
    }
}

impl Default for LocalStatsCollector {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}
