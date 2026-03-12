//! Client-side statistics types for business analytics.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;

/// Client statistics payload for upload to central server.
#[derive(Debug, serde::Serialize)]
pub struct ClientStatsPayload {
    pub client_id: String,
    pub timestamp: u64,
    pub security: SecurityDetails,
    pub business: BusinessStats,
    pub performance: PerformanceStats,
    pub resources: ResourceStats,
}

impl ClientStatsPayload {
    pub fn new(client_id: String, security: SecurityDetails) -> Self {
        Self {
            client_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            security,
            business: BusinessStats::new(),
            performance: PerformanceStats::new(),
            resources: ResourceStats::new(),
        }
    }
}

/// Security details for client validation.
#[derive(Debug, serde::Serialize)]
pub struct SecurityDetails {
    pub license_valid: bool,
    pub hardware_fingerprint: String,
    pub client_version: String,
}

impl SecurityDetails {
    pub fn new(license_valid: bool, hardware_fingerprint: String, client_version: String) -> Self {
        Self {
            license_valid,
            hardware_fingerprint,
            client_version,
        }
    }
}

/// Business statistics for tracking usage and engagement.
#[derive(Debug, serde::Serialize)]
pub struct BusinessStats {
    pub api_calls: AtomicU64,
    pub successful_calls: AtomicU64,
    pub failed_calls: AtomicU64,
    pub feature_usage: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
}

impl Default for BusinessStats {
    fn default() -> Self {
        Self::new()
    }
}

impl BusinessStats {
    pub fn new() -> Self {
        Self {
            api_calls: AtomicU64::new(0),
            successful_calls: AtomicU64::new(0),
            failed_calls: AtomicU64::new(0),
            feature_usage: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[inline]
    pub fn increment_api_calls(&self) {
        self.api_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_successful_calls(&self) {
        self.successful_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_failed_calls(&self) {
        self.failed_calls.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_feature_usage(&self, feature: &str) {
        let mut usage = self.feature_usage.write().unwrap_or_else(|e| e.into_inner());
        let counter = usage.entry(feature.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_api_calls(&self) -> u64 {
        self.api_calls.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_successful_calls(&self) -> u64 {
        self.successful_calls.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_failed_calls(&self) -> u64 {
        self.failed_calls.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_feature_usage(&self, feature: &str) -> u64 {
        let usage = self.feature_usage.read().unwrap_or_else(|e| e.into_inner());
        usage.get(feature).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }
}

/// Performance statistics for tracking response times and throughput.
#[derive(Debug, serde::Serialize)]
pub struct PerformanceStats {
    pub response_times: std::sync::Arc<std::sync::RwLock<VecDeque<u64>>>,
    pub throughput: AtomicU64,
    pub error_counts: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceStats {
    pub fn new() -> Self {
        Self {
            response_times: std::sync::Arc::new(std::sync::RwLock::new(VecDeque::new())),
            throughput: AtomicU64::new(0),
            error_counts: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[inline]
    pub fn record_response_time(&self, _metric_key: &str, duration_ms: u64) {
        let mut times = self.response_times.write().unwrap_or_else(|e| e.into_inner());
        times.push_back(duration_ms);
        
        if times.len() > 1000 {
            times.pop_front();
        }
    }

    #[inline]
    pub fn increment_throughput(&self, _metric_key: &str) {
        self.throughput.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_error(&self, metric_key: &str) {
        let mut errors = self.error_counts.write().unwrap_or_else(|e| e.into_inner());
        let counter = errors.entry(metric_key.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_avg_response_time(&self) -> Option<f64> {
        let times = self.response_times.read().unwrap_or_else(|e| e.into_inner());
        if times.is_empty() {
            None
        } else {
            let sum: u64 = times.iter().sum();
            Some(sum as f64 / times.len() as f64)
        }
    }

    #[inline]
    pub fn get_throughput(&self) -> u64 {
        self.throughput.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_error_count(&self, error_type: &str) -> u64 {
        let errors = self.error_counts.read().unwrap_or_else(|e| e.into_inner());
        errors.get(error_type).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }
}

/// Resource usage statistics for monitoring system resources.
#[derive(Debug, serde::Serialize)]
pub struct ResourceStats {
    pub memory_usage: AtomicU64,
    pub cpu_usage: AtomicU64,
    pub disk_operations: AtomicU64,
    pub network_bytes: AtomicU64,
}

impl Default for ResourceStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceStats {
    pub fn new() -> Self {
        Self {
            memory_usage: AtomicU64::new(0),
            cpu_usage: AtomicU64::new(0),
            disk_operations: AtomicU64::new(0),
            network_bytes: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn set_memory_usage(&self, bytes: u64) {
        self.memory_usage.store(bytes, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_cpu_usage(&self, percentage: u64) {
        self.cpu_usage.store(percentage, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_disk_operations(&self, count: u64) {
        self.disk_operations.fetch_add(count, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_network_bytes(&self, bytes: u64) {
        self.network_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_memory_usage(&self) -> u64 {
        self.memory_usage.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_cpu_usage(&self) -> u64 {
        self.cpu_usage.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_disk_operations(&self) -> u64 {
        self.disk_operations.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_network_bytes(&self) -> u64 {
        self.network_bytes.load(Ordering::Relaxed)
    }
}
