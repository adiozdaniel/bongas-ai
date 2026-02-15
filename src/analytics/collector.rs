//! Local statistics collector for client-side metrics.
//!
//! Provides in-memory collection of business statistics without database dependencies.
//! Uses existing resilience patterns for robust operation.

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
    business: Arc<BusinessStats>,
    
    /// Performance statistics.
    performance: Arc<PerformanceStats>,
    
    /// Resource statistics.
    resources: Arc<ResourceStats>,
    
    /// Collection interval for periodic updates.
    collection_interval: Duration,
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
        // In a real implementation, this would collect actual system metrics
        // For now, we'll simulate some basic resource usage
        
        // Simulate memory usage (in bytes)
        let memory_mb = 100 + (rand::random::<u64>() % 100);
        self.resources.set_memory_usage(memory_mb * 1024 * 1024);
        
        // Simulate CPU usage (0-100%)
        let cpu_usage = rand::random::<u64>() % 100;
        self.resources.set_cpu_usage(cpu_usage);
        
        // Simulate disk operations
        self.resources.increment_disk_operations(rand::random::<u64>() % 10);
        
        // Simulate network bytes
        self.resources.increment_network_bytes(rand::random::<u64>() % 1000);
    }

    /// Update performance statistics.
    async fn update_performance_stats(&self) {
        // In a real implementation, this would collect actual performance metrics
        // For now, we'll simulate some basic performance data
        
        // Simulate response times
        let response_time = 50 + (rand::random::<u64>() % 200);
        self.performance.record_response_time("simulated", response_time);
        
        // Simulate throughput
        self.performance.increment_throughput("simulated");
        
        // Simulate occasional errors
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