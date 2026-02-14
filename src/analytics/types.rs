//! Core types for the analytics module.
//!
//! Defines business metrics, user behavior tracking, and performance monitoring
//! types for application-level observability.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

// ─── Business Metrics ───────────────────────────────────────────────────────

/// Business metrics for tracking KPIs and user engagement.
#[derive(Debug)]
pub struct BusinessMetrics {
    /// Total number of active users.
    active_users: AtomicU64,
    
    /// Total number of new users.
    new_users: AtomicU64,
    
    /// Total revenue (in cents).
    revenue: AtomicU64,
    
    /// Conversion rate (0.0 to 1.0).
    conversion_rate: AtomicU64, // Stored as u64 with 6 decimal places
    
    /// Retention rate (0.0 to 1.0).
    retention_rate: AtomicU64, // Stored as u64 with 6 decimal places
}

impl BusinessMetrics {
    pub fn new() -> Self {
        Self {
            active_users: AtomicU64::new(0),
            new_users: AtomicU64::new(0),
            revenue: AtomicU64::new(0),
            conversion_rate: AtomicU64::new(0),
            retention_rate: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn increment_active_users(&self) {
        self.active_users.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_new_users(&self) {
        self.new_users.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn add_revenue(&self, amount_cents: u64) {
        self.revenue.fetch_add(amount_cents, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_conversion_rate(&self, rate: f64) {
        let rate_scaled = (rate * 1_000_000.0) as u64;
        self.conversion_rate.store(rate_scaled, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_retention_rate(&self, rate: f64) {
        let rate_scaled = (rate * 1_000_000.0) as u64;
        self.retention_rate.store(rate_scaled, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_active_users(&self) -> u64 {
        self.active_users.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_new_users(&self) -> u64 {
        self.new_users.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_revenue(&self) -> u64 {
        self.revenue.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_conversion_rate(&self) -> f64 {
        let rate_scaled = self.conversion_rate.load(Ordering::Relaxed);
        rate_scaled as f64 / 1_000_000.0
    }

    #[inline]
    pub fn get_retention_rate(&self) -> f64 {
        let rate_scaled = self.retention_rate.load(Ordering::Relaxed);
        rate_scaled as f64 / 1_000_000.0
    }
}

impl Default for BusinessMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ─── User Behavior ──────────────────────────────────────────────────────────

/// User behavior metrics for tracking feature usage and engagement.
#[derive(Debug)]
pub struct UserBehavior {
    /// Total page views.
    page_views: AtomicU64,
    
    /// Feature usage counts.
    feature_usage: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
    
    /// Session duration tracking.
    session_duration: AtomicU64, // Total milliseconds
    session_count: AtomicU64,
}

impl UserBehavior {
    pub fn new() -> Self {
        Self {
            page_views: AtomicU64::new(0),
            feature_usage: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            session_duration: AtomicU64::new(0),
            session_count: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn increment_page_views(&self) {
        self.page_views.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_feature_usage(&self, feature: &str) {
        let mut usage = self.feature_usage.write().unwrap();
        let counter = usage.entry(feature.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_session(&self, duration: Duration) {
        self.session_duration.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.session_count.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_page_views(&self) -> u64 {
        self.page_views.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_feature_usage(&self, feature: &str) -> u64 {
        let usage = self.feature_usage.read().unwrap();
        usage.get(feature).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn get_avg_session_duration(&self) -> Duration {
        let total_duration = self.session_duration.load(Ordering::Relaxed);
        let count = self.session_count.load(Ordering::Relaxed);
        
        if count == 0 {
            Duration::from_secs(0)
        } else {
            Duration::from_millis(total_duration / count)
        }
    }
}

impl Default for UserBehavior {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Performance Metrics ────────────────────────────────────────────────────

/// Performance metrics for API and system monitoring.
#[derive(Debug)]
pub struct PerformanceMetrics {
    /// API response time tracking.
    response_times: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, Vec<u64>>>>,
    
    /// Throughput tracking.
    throughput: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
    
    /// Error rates.
    error_counts: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, AtomicU64>>>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            response_times: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            throughput: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
            error_counts: std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    #[inline]
    pub fn record_response_time(&self, endpoint: &str, duration_ms: u64) {
        let mut times = self.response_times.write().unwrap();
        let entry = times.entry(endpoint.to_string()).or_insert_with(Vec::new);
        entry.push(duration_ms);
        
        // Keep only last 1000 measurements to prevent memory growth
        if entry.len() > 1000 {
            entry.drain(0..entry.len() - 1000);
        }
    }

    #[inline]
    pub fn increment_throughput(&self, endpoint: &str) {
        let mut throughput = self.throughput.write().unwrap();
        let counter = throughput.entry(endpoint.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_error(&self, endpoint: &str) {
        let mut errors = self.error_counts.write().unwrap();
        let counter = errors.entry(endpoint.to_string()).or_insert_with(|| AtomicU64::new(0));
        counter.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn get_avg_response_time(&self, endpoint: &str) -> Option<f64> {
        let times = self.response_times.read().unwrap();
        let measurements = times.get(endpoint)?;
        
        if measurements.is_empty() {
            None
        } else {
            let sum: u64 = measurements.iter().sum();
            Some(sum as f64 / measurements.len() as f64)
        }
    }

    #[inline]
    pub fn get_throughput(&self, endpoint: &str) -> u64 {
        let throughput = self.throughput.read().unwrap();
        throughput.get(endpoint).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }

    #[inline]
    pub fn get_error_count(&self, endpoint: &str) -> u64 {
        let errors = self.error_counts.read().unwrap();
        errors.get(endpoint).map_or(0, |counter| counter.load(Ordering::Relaxed))
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Resource Metrics ───────────────────────────────────────────────────────

/// Resource usage metrics for system monitoring.
#[derive(Debug)]
pub struct ResourceMetrics {
    /// Memory usage in bytes.
    memory_usage: AtomicU64,
    
    /// CPU usage percentage (0-100).
    cpu_usage: AtomicU64,
    
    /// Disk I/O operations.
    disk_reads: AtomicU64,
    disk_writes: AtomicU64,
    
    /// Network I/O.
    network_bytes_sent: AtomicU64,
    network_bytes_received: AtomicU64,
}

impl ResourceMetrics {
    pub fn new() -> Self {
        Self {
            memory_usage: AtomicU64::new(0),
            cpu_usage: AtomicU64::new(0),
            disk_reads: AtomicU64::new(0),
            disk_writes: AtomicU64::new(0),
            network_bytes_sent: AtomicU64::new(0),
            network_bytes_received: AtomicU64::new(0),
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
    pub fn increment_disk_reads(&self, count: u64) {
        self.disk_reads.fetch_add(count, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_disk_writes(&self, count: u64) {
        self.disk_writes.fetch_add(count, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_network_sent(&self, bytes: u64) {
        self.network_bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    #[inline]
    pub fn increment_network_received(&self, bytes: u64) {
        self.network_bytes_received.fetch_add(bytes, Ordering::Relaxed);
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
    pub fn get_disk_reads(&self) -> u64 {
        self.disk_reads.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_disk_writes(&self) -> u64 {
        self.disk_writes.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_network_sent(&self) -> u64 {
        self.network_bytes_sent.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn get_network_received(&self) -> u64 {
        self.network_bytes_received.load(Ordering::Relaxed)
    }
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self::new()
    }
}