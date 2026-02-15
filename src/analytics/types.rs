//! Client-side statistics types for business analytics.
//!
//! Provides types for collecting and uploading client-side statistics
//! including usage metrics, performance data, and security information.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Client statistics payload for upload to central server.
#[derive(Debug, serde::Serialize)]
pub struct ClientStatsPayload {
    /// Unique client identifier.
    pub client_id: String,
    
    /// Timestamp of when statistics were collected.
    pub timestamp: u64,
    
    /// Security details for validation.
    pub security: SecurityDetails,
    
    /// Business metrics.
    pub business: BusinessStats,
    
    /// Performance metrics.
    pub performance: PerformanceStats,
    
    /// Resource usage metrics.
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
    /// License status.
    pub license_valid: bool,
    
    /// Hardware fingerprint.
    pub hardware_fingerprint: String,
    
    /// Client version.
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
    /// Total API calls made.
    pub api_calls: AtomicU64,
    
    /// Successful API calls.
    pub successful_calls: AtomicU64,
    
    /// Failed API calls.
    pub failed_calls: AtomicU64,
    
    /// Feature usage counts.
    pub feature_usage: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
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
        let mut usage = self.feature_usage.write().unwrap();
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
        let usage = self.feature_usage.read().unwrap();
        usage.get(feature).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }
}

/// Performance statistics for tracking response times and throughput.
#[derive(Debug, serde::Serialize)]
pub struct PerformanceStats {
    /// Response time measurements.
    pub response_times: std::sync::Arc<std::sync::RwLock<Vec<u64>>>,
    
    /// Throughput measurements.
    pub throughput: AtomicU64,
    
    /// Error counts by type.
    pub error_counts: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
}

impl PerformanceStats {
    pub fn new() -> Self {
        Self {
            response_times: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
            throughput: AtomicU64::new(0),
            error_counts: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[inline]
    pub fn record_response_time(&self, _metric_key: &str, duration_ms: u64) {
        let mut times = self.response_times.write().unwrap();
        times.push(duration_ms);
        
        // Keep only last 1000 measurements to prevent memory growth
        if times.len() > 1000 {
            let new_len = times.len() - 1000;
            times.drain(0..new_len);
        }
    }

    #[inline]
    pub fn increment_throughput(&self, _metric_key: &str) {
        self.throughput.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_error(&self, metric_key: &str) {
        let mut errors = self.error_counts.write().unwrap();
        let counter = errors.entry(metric_key.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_avg_response_time(&self) -> Option<f64> {
        let times = self.response_times.read().unwrap();
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
        let errors = self.error_counts.read().unwrap();
        errors.get(error_type).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }
}

/// Resource usage statistics for monitoring system resources.
#[derive(Debug, serde::Serialize)]
pub struct ResourceStats {
    /// Memory usage in bytes.
    pub memory_usage: AtomicU64,
    
    /// CPU usage percentage (0-100).
    pub cpu_usage: AtomicU64,
    
    /// Disk I/O operations.
    pub disk_operations: AtomicU64,
    
    /// Network I/O bytes.
    pub network_bytes: AtomicU64,
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