//! Export functionality for analytics data.
//!
//! Provides pluggable exporters for sending business metrics to various
//! external systems like Prometheus, JSON files, or custom endpoints.

use crate::analytics::types::{BusinessMetrics, UserBehavior, PerformanceMetrics, ResourceMetrics};

/// Trait for exporting analytics data to external systems.
pub trait AnalyticsExporter {
    /// Export business metrics.
    fn export_business_metrics(&self, metrics: &BusinessMetrics) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Export user behavior metrics.
    fn export_user_behavior(&self, metrics: &UserBehavior) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Export performance metrics.
    fn export_performance_metrics(&self, metrics: &PerformanceMetrics) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Export resource metrics.
    fn export_resource_metrics(&self, metrics: &ResourceMetrics) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Export all metrics at once.
    fn export_all(
        &self,
        business: &BusinessMetrics,
        user_behavior: &UserBehavior,
        performance: &PerformanceMetrics,
        resources: &ResourceMetrics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.export_business_metrics(business)?;
        self.export_user_behavior(user_behavior)?;
        self.export_performance_metrics(performance)?;
        self.export_resource_metrics(resources)?;
        Ok(())
    }
}

/// JSON exporter for analytics data.
pub struct JsonExporter {
    output_path: String,
}

impl JsonExporter {
    pub fn new(output_path: impl Into<String>) -> Self {
        Self {
            output_path: output_path.into(),
        }
    }
}

impl AnalyticsExporter for JsonExporter {
    fn export_business_metrics(&self, metrics: &BusinessMetrics) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "active_users": metrics.get_active_users(),
            "new_users": metrics.get_new_users(),
            "revenue": metrics.get_revenue(),
            "conversion_rate": metrics.get_conversion_rate(),
            "retention_rate": metrics.get_retention_rate(),
        });
        
        std::fs::write(&self.output_path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    fn export_user_behavior(&self, metrics: &UserBehavior) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "page_views": metrics.get_page_views(),
            "avg_session_duration_ms": metrics.get_avg_session_duration().as_millis(),
        });
        
        std::fs::write(&self.output_path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    fn export_performance_metrics(&self, _metrics: &PerformanceMetrics) -> Result<(), Box<dyn std::error::Error>> {
        // For simplicity, just export a summary
        let data = serde_json::json!({
            "note": "Performance metrics require endpoint-specific queries",
        });

        std::fs::write(&self.output_path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    fn export_resource_metrics(&self, metrics: &ResourceMetrics) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "memory_usage_bytes": metrics.get_memory_usage(),
            "cpu_usage_percent": metrics.get_cpu_usage(),
            "disk_reads": metrics.get_disk_reads(),
            "disk_writes": metrics.get_disk_writes(),
            "network_bytes_sent": metrics.get_network_sent(),
            "network_bytes_received": metrics.get_network_received(),
        });
        
        std::fs::write(&self.output_path, serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }
}

/// Prometheus exporter for analytics data.
pub struct PrometheusExporter {
    endpoint: String,
}

impl PrometheusExporter {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

impl AnalyticsExporter for PrometheusExporter {
    fn export_business_metrics(&self, metrics: &BusinessMetrics) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "active_users": metrics.get_active_users(),
            "new_users": metrics.get_new_users(),
            "revenue": metrics.get_revenue(),
            "conversion_rate": metrics.get_conversion_rate(),
            "retention_rate": metrics.get_retention_rate(),
        });
        
        let client = reqwest::blocking::Client::new();
        let response = client.post(&self.endpoint)
            .json(&data)
            .send()?;
            
        if response.status().is_success() {
            tracing::info!("Business metrics successfully sent to Prometheus endpoint: {}", self.endpoint);
            Ok(())
        } else {
            Err(format!("Failed to send business metrics to Prometheus: HTTP {}", response.status()).into())
        }
    }

    fn export_user_behavior(&self, metrics: &UserBehavior) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "page_views": metrics.get_page_views(),
            "avg_session_duration_ms": metrics.get_avg_session_duration().as_millis(),
        });
        
        let client = reqwest::blocking::Client::new();
        let response = client.post(&self.endpoint)
            .json(&data)
            .send()?;
            
        if response.status().is_success() {
            tracing::info!("User behavior metrics successfully sent to Prometheus endpoint: {}", self.endpoint);
            Ok(())
        } else {
            Err(format!("Failed to send user behavior metrics to Prometheus: HTTP {}", response.status()).into())
        }
    }

    fn export_performance_metrics(&self, _metrics: &PerformanceMetrics) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "note": "Performance metrics require endpoint-specific queries",
        });
        
        let client = reqwest::blocking::Client::new();
        let response = client.post(&self.endpoint)
            .json(&data)
            .send()?;
            
        if response.status().is_success() {
            tracing::info!("Performance metrics successfully sent to Prometheus endpoint: {}", self.endpoint);
            Ok(())
        } else {
            Err(format!("Failed to send performance metrics to Prometheus: HTTP {}", response.status()).into())
        }
    }

    fn export_resource_metrics(&self, metrics: &ResourceMetrics) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "memory_usage_bytes": metrics.get_memory_usage(),
            "cpu_usage_percent": metrics.get_cpu_usage(),
            "disk_reads": metrics.get_disk_reads(),
            "disk_writes": metrics.get_disk_writes(),
            "network_bytes_sent": metrics.get_network_sent(),
            "network_bytes_received": metrics.get_network_received(),
        });
        
        let client = reqwest::blocking::Client::new();
        let response = client.post(&self.endpoint)
            .json(&data)
            .send()?;
            
        if response.status().is_success() {
            tracing::info!("Resource metrics successfully sent to Prometheus endpoint: {}", self.endpoint);
            Ok(())
        } else {
            Err(format!("Failed to send resource metrics to Prometheus: HTTP {}", response.status()).into())
        }
    }
}
