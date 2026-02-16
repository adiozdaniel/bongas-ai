use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, debug};
use chrono::Timelike;

use crate::engine::engine::BongasEngine;

/// Phase 6: Predictive Cache Warmer
/// 
/// Predicts user arrival times based on historical interaction patterns
/// and pre-warms their personalized recommendations.
pub struct PredictiveWarmer {
    engine: Arc<BongasEngine>,
    warm_scenarios: Vec<String>,
}

impl PredictiveWarmer {
    pub fn new(engine: Arc<BongasEngine>, warm_scenarios: Vec<String>) -> Self {
        Self {
            engine,
            warm_scenarios,
        }
    }

    /// Start the predictive warming background task.
    pub async fn start(self: Arc<Self>) {
        // Run every hour to predict the NEXT hour's arrivals
        let mut ticker = interval(Duration::from_secs(3600));

        loop {
            ticker.tick().await;
            
            let next_hour = (chrono::Utc::now().hour() + 1) % 24;
            info!(next_hour, "Starting predictive cache warming cycle");

            if let Err(e) = self.warm_next_arrivals(next_hour).await {
                warn!(error = %e, "Predictive warming cycle failed");
            }
        }
    }

    async fn warm_next_arrivals(&self, target_hour: u32) -> Result<()> {
        use crate::resilience::{ResilienceMetricsCollector, MetricsRegistry, ResilienceConfig};
        
        // 1. Get users likely to arrive in the target hour
        let resilience_metrics = Arc::new(ResilienceMetricsCollector::new(
            Arc::new(MetricsRegistry::new(ResilienceConfig::default())),
        ));

        let interaction_repo = crate::db::repositories::interaction_repository::InteractionRepository::new(
            self.engine.item_feature_service.pool().clone(),
            resilience_metrics,
        );

        let likely_users = interaction_repo.get_likely_arrivals(target_hour).await?;
        info!(user_count = likely_users.len(), target_hour, "Identified likely arrivals");

        // 2. Trigger warming for each user + scenario
        for user_id in likely_users {
            for scenario in &self.warm_scenarios {
                let engine = self.engine.clone();
                let scenario_slug = scenario.clone();
                
                // Use tokio::spawn to parallelize individual user warming
                tokio::spawn(async move {
                    debug!(user_id, scenario = %scenario_slug, "Predictively warming user cache");
                    
                    // Execute scenario with dummy context params
                    let context_params = serde_json::json!({
                        "predictive_warm": true
                    });
                    
                    if let Err(e) = engine.execute_scenario(&scenario_slug, Some(user_id), context_params).await {
                        warn!(user_id, scenario = %scenario_slug, error = %e, "Predictive warm execution failed");
                    }
                });
            }
        }

        Ok(())
    }
}
