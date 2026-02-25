use std::sync::Arc;
use anyhow::Result;
use crate::engine::engine::types::*;
use crate::ingestion::metrics::IngestionHealth;
use crate::db::repositories::feature_repository::FeatureRepository;
use crate::db::repositories::cache_repository::CacheRepository;
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::engine::staleness_engine::StalenessEngine;
use crate::cache::metrics::CacheMetricsSnapshot;
use crate::engine::scenario_factory::ScenarioFactory;

impl BongasEngine {
    /// Proxy: Execute scenario and return recommendations
    pub async fn execute_scenario(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
    ) -> Result<Vec<RecommendationItem>> {
        let (items, _) = self.execution.execute_scenario_with_stats_contextual(
            scenario_slug, user_id, None, None, None, context_params, None
        ).await?;
        Ok(items)
    }

    /// Proxy: Execute scenario with execution stats and persona context
    pub async fn execute_scenario_with_stats_contextual(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        profile_id: Option<String>,
        maturity_rating: Option<String>,
        device_type: Option<String>,
        context_params: serde_json::Value,
        limit: Option<usize>,
    ) -> Result<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        self.execution.execute_scenario_with_stats_contextual(
            scenario_slug, user_id, profile_id, maturity_rating, device_type, context_params, limit
        ).await
    }

    /// Compatibility proxy: execute_scenario_with_stats
    pub async fn execute_scenario_with_stats(
        &self,
        scenario_slug: &str,
        user_id: Option<i32>,
        context_params: serde_json::Value,
        limit: Option<usize>,
    ) -> Result<(Vec<RecommendationItem>, ScenarioExecutionStats)> {
        self.execution.execute_scenario_with_stats_contextual(
            scenario_slug, user_id, None, None, None, context_params, limit
        ).await
    }

    /// Proxy: Reload all scenarios
    pub async fn reload_scenarios(&self) -> Result<usize> {
        self.scenarios.reload_scenarios().await
    }

    /// Proxy: Reload a single scenario
    pub async fn reload_scenario(&self, slug: &str) -> Result<bool> {
        self.scenarios.reload_scenario(slug).await
    }

    /// Proxy: Remove a scenario
    pub async fn remove_scenario(&self, slug: &str) {
        self.scenarios.remove_scenario(slug).await
    }

    /// Proxy: List loaded scenario slugs
    pub async fn list_scenarios(&self) -> Vec<String> {
        let scenarios = self.scenarios.scenarios.read().await;
        scenarios.keys().cloned().collect()
    }

    pub async fn ingestion_health(&self) -> IngestionHealth {
        let manager = self.ingestion_manager.read().await;
        manager.health().await
    }

    pub fn get_hit_rate(&self) -> f64 {
        self.staging_manager.get_hit_rate()
    }

    pub fn get_cache_stats(&self) -> CacheMetricsSnapshot {
        self.cache_manager.metrics()
    }

    pub async fn reload_models(&self) -> Result<usize> {
        self.execution.model_loader.reload_all().await
    }

    pub async fn model_count(&self) -> usize {
        self.execution.model_loader.loaded_count().await
    }

    pub async fn get_security_status(&self) -> SecurityStatus {
        SecurityStatus {
            validated: self.security_manager.is_validated().await,
            security_enabled: true,
            layers_configured: 8,
        }
    }

    pub fn feature_repo(&self) -> Arc<FeatureRepository> {
        self.execution.feature_repo.clone()
    }

    pub fn cache_repo(&self) -> Arc<CacheRepository> {
        self.execution.cache_repo.clone()
    }

    pub fn scenario_factory(&self) -> Arc<ScenarioFactory> {
        self.scenarios.scenario_factory.clone()
    }

    pub fn circuit_breaker_registry(&self) -> Arc<CircuitBreakerRegistry> {
        self.execution.circuit_breaker_registry.clone()
    }

    pub fn staleness_engine(&self) -> Arc<StalenessEngine> {
        self.staleness_engine.clone()
    }
}
