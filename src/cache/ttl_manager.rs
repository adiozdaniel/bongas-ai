use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

use crate::db::repositories::cache_repository::{CacheRepository, CacheStats as RepoCacheStats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioTTLConfig {
    pub scenario_slug: String,
    pub base_l1_ttl_seconds: i32,
    pub base_l2_ttl_seconds: i32,
    pub adaptive_enabled: bool,
    pub staleness_threshold_hours: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TTLManager {
    cache_repo: Arc<CacheRepository>,
    scenario_configs: HashMap<String, ScenarioTTLConfig>,
}

impl TTLManager {
    pub fn new(cache_repo: Arc<CacheRepository>) -> Self {
        Self {
            cache_repo,
            scenario_configs: HashMap::new(),
        }
    }

    /// Register scenario TTL configuration
    pub fn register_scenario(&mut self, config: ScenarioTTLConfig) {
        let scenario_slug = config.scenario_slug.clone();
        self.scenario_configs.insert(scenario_slug.clone(), config.clone());
        info!(
            scenario_slug = scenario_slug,
            l1_ttl = config.base_l1_ttl_seconds,
            l2_ttl = config.base_l2_ttl_seconds,
            adaptive = config.adaptive_enabled,
            "Registered scenario TTL configuration"
        );
    }

    /// Get TTL for a scenario
    pub fn get_ttl_for_scenario(&self, scenario_slug: &str) -> (i32, i32) {
        let config = self.scenario_configs.get(scenario_slug);

        match config {
            Some(config) => {
                if config.adaptive_enabled {
                    self.calculate_adaptive_ttl(scenario_slug, config)
                } else {
                    (config.base_l1_ttl_seconds, config.base_l2_ttl_seconds)
                }
            }
            None => {
                // Default TTLs
                (300, 3600) // 5 min L1, 1 hour L2
            }
        }
    }

    /// Calculate adaptive TTL based on scenario characteristics
    fn calculate_adaptive_ttl(&self, scenario_slug: &str, config: &ScenarioTTLConfig) -> (i32, i32) {
        // For now, return base TTLs
        // In future, could implement logic based on:
        // - Content freshness requirements
        // - User interaction patterns
        // - Content type (e.g., trending vs personalized)
        // - Time of day
        
        info!(
            scenario_slug = scenario_slug,
            "Using adaptive TTL calculation (placeholder)"
        );

        (config.base_l1_ttl_seconds, config.base_l2_ttl_seconds)
    }

    /// Get cache statistics for TTL optimization
    pub async fn get_cache_stats(&self, scenario_slug: &str) -> Result<CacheStats> {
        let repo_stats = self.cache_repo.get_cache_stats(scenario_slug).await?;
        Ok(CacheStats {
            total_entries: repo_stats.total_entries,
            active_entries: repo_stats.active_entries,
            stale_entries: repo_stats.stale_entries,
            total_hits: repo_stats.total_hits,
        })
    }

    /// Cleanup expired entries with TTL management
    pub async fn cleanup_expired_with_ttl(&self) -> Result<u64> {
        // Get all scenarios and their TTLs
        let mut total_cleaned = 0;

        for (scenario_slug, config) in &self.scenario_configs {
            // Check for entries that should be expired based on TTL
            let stats = self.get_cache_stats(scenario_slug).await?;
            
            if stats.stale_entries > 0 {
                warn!(
                    scenario_slug = scenario_slug,
                    stale_entries = stats.stale_entries,
                    "Found stale cache entries"
                );
            }

            // Cleanup expired entries
            let cleaned = self.cache_repo.cleanup_expired().await?;
            total_cleaned += cleaned;
        }

        info!(total_cleaned = total_cleaned, "TTL-based cleanup completed");
        Ok(total_cleaned)
    }

    /// Optimize TTLs based on cache performance
    pub async fn optimize_ttls(&mut self) -> Result<()> {
        let scenario_slugs: Vec<String> = self.scenario_configs.keys().cloned().collect();
        
        for scenario_slug in scenario_slugs {
            let stats = self.get_cache_stats(&scenario_slug).await?;

            // Calculate hit rate
            let hit_rate = if stats.total_hits > 0 {
                stats.total_hits as f64 / (stats.total_entries as f64) 
            } else {
                0.0
            };

            // Adjust TTLs based on hit rate and staleness
            if let Some(config) = self.scenario_configs.get_mut(&scenario_slug) {
                if hit_rate < 0.5 && config.base_l1_ttl_seconds > 60 {
                    // Low hit rate, reduce TTL
                    config.base_l1_ttl_seconds = (config.base_l1_ttl_seconds as f64 * 0.8) as i32;
                    config.base_l2_ttl_seconds = (config.base_l2_ttl_seconds as f64 * 0.8) as i32;
                    
                    info!(
                        scenario_slug = scenario_slug,
                        new_l1_ttl = config.base_l1_ttl_seconds,
                        new_l2_ttl = config.base_l2_ttl_seconds,
                        "Reduced TTL due to low hit rate"
                    );
                } else if hit_rate > 0.8 && config.base_l1_ttl_seconds < 1800 {
                    // High hit rate, increase TTL
                    config.base_l1_ttl_seconds = (config.base_l1_ttl_seconds as f64 * 1.2) as i32;
                    config.base_l2_ttl_seconds = (config.base_l2_ttl_seconds as f64 * 1.2) as i32;
                    
                    info!(
                        scenario_slug = scenario_slug,
                        new_l1_ttl = config.base_l1_ttl_seconds,
                        new_l2_ttl = config.base_l2_ttl_seconds,
                        "Increased TTL due to high hit rate"
                    );
                }
            }
        }

        Ok(())
    }

    /// Start TTL optimization background task
    pub fn start_ttl_optimizer(self: Arc<Self>, interval_minutes: u64) {
        let ttl_manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_minutes * 60));

            loop {
                interval.tick().await;

                // For now, just cleanup expired entries
                if let Err(e) = ttl_manager.cleanup_expired_with_ttl().await {
                    warn!(error = ?e, "TTL-based cleanup failed");
                }
            }
        });

        info!(interval_minutes = interval_minutes, "TTL optimizer started");
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub total_entries: i64,
    pub active_entries: i64,
    pub stale_entries: i64,
    pub total_hits: i64,
}