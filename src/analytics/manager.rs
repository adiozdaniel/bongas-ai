//! Analytics manager for coordinating business metrics collection.
//!
//! Provides a central point for collecting, aggregating, and exporting
//! business analytics data across the application.

use std::sync::Arc;
use std::time::Duration;

use crate::analytics::config::AnalyticsConfig;
use crate::analytics::exporter::{AnalyticsExporter, JsonExporter};
use crate::analytics::types::{BusinessMetrics, UserBehavior, PerformanceMetrics, ResourceMetrics};

/// Central manager for business analytics.
pub struct AnalyticsManager {
    config: AnalyticsConfig,
    business_metrics: Arc<BusinessMetrics>,
    user_behavior: Arc<UserBehavior>,
    performance_metrics: Arc<PerformanceMetrics>,
    resource_metrics: Arc<ResourceMetrics>,
    exporter: Option<Arc<dyn AnalyticsExporter + Send + Sync>>,
    export_task: tokio::task::JoinHandle<()>,
}

impl AnalyticsManager {
    /// Create a new analytics manager with the given configuration.
    pub fn new(config: AnalyticsConfig) -> Self {
        let business_metrics = Arc::new(BusinessMetrics::new());
        let user_behavior = Arc::new(UserBehavior::new());
        let performance_metrics = Arc::new(PerformanceMetrics::new());
        let resource_metrics = Arc::new(ResourceMetrics::new());
        
        let exporter = match config.export_interval() {
            Some(_interval) => {
                // For now, just use a JSON exporter as an example
                Some(Arc::new(JsonExporter::new("analytics_export.json")) as Arc<dyn AnalyticsExporter + Send + Sync>)
            }
            None => None,
        };

        let business_metrics_clone = Arc::clone(&business_metrics);
        let user_behavior_clone = Arc::clone(&user_behavior);
        let performance_metrics_clone = Arc::clone(&performance_metrics);
        let resource_metrics_clone = Arc::clone(&resource_metrics);
        let exporter_clone = exporter.clone();

        let export_task = tokio::spawn(async move {
            if let Some(exporter) = exporter_clone {
                let mut interval = tokio::time::interval(Duration::from_secs(60)); // Default 1-minute interval
                
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = exporter.export_all(
                        &business_metrics_clone,
                        &user_behavior_clone,
                        &performance_metrics_clone,
                        &resource_metrics_clone,
                    ) {
                        tracing::error!("Failed to export analytics: {}", e);
                    }
                }
            }
        });

        Self {
            config,
            business_metrics,
            user_behavior,
            performance_metrics,
            resource_metrics,
            exporter,
            export_task,
        }
    }

    /// Get the business metrics instance.
    pub fn business_metrics(&self) -> Arc<BusinessMetrics> {
        Arc::clone(&self.business_metrics)
    }

    /// Get the user behavior metrics instance.
    pub fn user_behavior(&self) -> Arc<UserBehavior> {
        Arc::clone(&self.user_behavior)
    }

    /// Get the performance metrics instance.
    pub fn performance_metrics(&self) -> Arc<PerformanceMetrics> {
        Arc::clone(&self.performance_metrics)
    }

    /// Get the resource metrics instance.
    pub fn resource_metrics(&self) -> Arc<ResourceMetrics> {
        Arc::clone(&self.resource_metrics)
    }

    /// Shutdown the analytics manager.
    pub async fn shutdown(&self) {
        // We can't move out of self, so we need to handle this differently
        // For now, just abort the task without awaiting it
        self.export_task.abort();
    }
}

impl Default for AnalyticsManager {
    fn default() -> Self {
        Self::new(AnalyticsConfig::default())
    }
}