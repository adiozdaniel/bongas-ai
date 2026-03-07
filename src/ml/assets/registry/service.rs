//! Versioned model registry with health tracking, canary routing, and shadow mode.
//!
//! # Netflix Resilience Patterns
//! - **Canary Deployment**: Route % of traffic to new model version
//! - **Shadow Mode**: Score with new model but serve old model's results
//! - **Health Tracking**: Per-model health derived from circuit breaker state
//! - **Analytics**: Model serving distribution, canary success rates

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::{info, debug};

use crate::config::MlConfig;
use crate::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerId};
/// Model version state in the registry.
#[derive(Debug, Clone)]
pub struct ModelVersion {
    pub model_name: String,
    pub version: String,
    pub status: ModelStatus,
    pub registered_at: Instant,
    pub canary_percent: f64,
    pub shadow_mode: bool,
}

/// Model deployment status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    /// Fully serving production traffic.
    Active,
    /// Receiving canary traffic only.
    Canary,
    /// Running in shadow mode (score but don't serve).
    Shadow,
    /// Deprecated, no traffic.
    Deprecated,
}

/// Health summary for a model version.
#[derive(Debug, Clone)]
pub struct ModelHealth {
    pub model_name: String,
    pub version: String,
    pub status: ModelStatus,
    pub breaker_state: String,
    pub inference_count: u64,
    pub error_count: u64,
}

/// Versioned model registry with canary routing and shadow mode.
pub struct VersionedModelRegistry {
    versions: Arc<RwLock<HashMap<String, Vec<ModelVersion>>>>,
    config: MlConfig,
    analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
    breaker_registry: Arc<CircuitBreakerRegistry>,
}

impl VersionedModelRegistry {
    pub fn new(
        config: MlConfig,
        analytics: Option<Arc<crate::analytics::types::PerformanceStats>>,
        breaker_registry: Arc<CircuitBreakerRegistry>,
    ) -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            config,
            analytics,
            breaker_registry,
        }
    }

    /// Register a new model version.
    pub async fn register(
        &self,
        model_name: &str,
        version: &str,
        status: ModelStatus,
    ) {
        let model_version = ModelVersion {
            model_name: model_name.to_string(),
            version: version.to_string(),
            status: status.clone(),
            registered_at: Instant::now(),
            canary_percent: if status == ModelStatus::Canary {
                self.config.canary_traffic_percent
            } else {
                0.0
            },
            shadow_mode: status == ModelStatus::Shadow,
        };

        let mut versions = self.versions.write().await;
        let entries = versions.entry(model_name.to_string()).or_insert_with(Vec::new);
        entries.push(model_version);

        info!(
            model = %model_name,
            version = %version,
            status = ?status,
            "Model version registered"
        );

        if let Some(ref a) = self.analytics {
            a.increment_throughput("ml.registry.register");
        }
    }

    /// Select which model version to use based on canary routing.
    ///
    /// Uses a hash of the request context to deterministically route traffic.
    pub async fn select_version(
        &self,
        model_name: &str,
        routing_key: u64,
    ) -> Option<String> {
        let versions = self.versions.read().await;
        let entries = versions.get(model_name)?;

        if !self.config.canary_enabled {
            // Return the latest active version
            return entries.iter().rev()
                .find(|v| v.status == ModelStatus::Active)
                .map(|v| v.version.clone());
        }

        // Canary routing: use routing_key to deterministically select
        let canary = entries.iter().rev()
            .find(|v| v.status == ModelStatus::Canary);

        if let Some(canary_version) = canary {
            let bucket = (routing_key % 100) as f64;
            if bucket < canary_version.canary_percent {
                debug!(
                    model = %model_name,
                    version = %canary_version.version,
                    "Routing to canary model"
                );
                if let Some(ref a) = self.analytics {
                    a.increment_throughput("ml.registry.canary_routed");
                }
                return Some(canary_version.version.clone());
            }
        }

        // Default: latest active version
        entries.iter().rev()
            .find(|v| v.status == ModelStatus::Active)
            .map(|v| v.version.clone())
    }

    /// Get shadow model version if shadow mode is enabled.
    pub async fn get_shadow_version(&self, model_name: &str) -> Option<String> {
        if !self.config.shadow_mode_enabled {
            return None;
        }

        let versions = self.versions.read().await;
        let entries = versions.get(model_name)?;

        entries.iter().rev()
            .find(|v| v.status == ModelStatus::Shadow)
            .map(|v| v.version.clone())
    }

    /// Promote a canary version to active.
    pub async fn promote_canary(&self, model_name: &str, version: &str) {
        let mut versions = self.versions.write().await;
        if let Some(entries) = versions.get_mut(model_name) {
            // Deprecate old active versions
            for entry in entries.iter_mut() {
                if entry.status == ModelStatus::Active {
                    entry.status = ModelStatus::Deprecated;
                }
            }
            // Promote canary
            for entry in entries.iter_mut() {
                if entry.version == version && entry.status == ModelStatus::Canary {
                    entry.status = ModelStatus::Active;
                    entry.canary_percent = 0.0;
                    info!(
                        model = %model_name,
                        version = %version,
                        "Canary promoted to active"
                    );
                }
            }
        }

        if let Some(ref a) = self.analytics {
            a.increment_throughput("ml.registry.canary_promoted");
        }
    }

    /// Get all model versions for a model name.
    pub async fn get_versions(&self, model_name: &str) -> Vec<ModelVersion> {
        let versions = self.versions.read().await;
        versions.get(model_name).cloned().unwrap_or_default()
    }

    /// Get health summary for all models.
    pub async fn health_all(&self) -> Vec<ModelHealth> {
        let versions = self.versions.read().await;
        let mut health = Vec::new();

        for entries in versions.values() {
            for entry in entries {
                let breaker_id = CircuitBreakerId::new(format!("model.{}", entry.model_name));
                let breaker_state = self.breaker_registry
                    .get(&breaker_id)
                    .map(|b| format!("{:?}", b.current_state()))
                    .unwrap_or_else(|| "unknown".to_string());

                health.push(ModelHealth {
                    model_name: entry.model_name.clone(),
                    version: entry.version.clone(),
                    status: entry.status.clone(),
                    breaker_state,
                    inference_count: 0,
                    error_count: 0,
                });
            }
        }

        health
    }
}
