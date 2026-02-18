use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tokio::sync::Semaphore;
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
    concurrency_limit: Arc<Semaphore>,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
}

impl PredictiveWarmer {
    pub fn new(
        engine: Arc<BongasEngine>, 
        warm_scenarios: Vec<String>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Self {
        Self {
            engine,
            warm_scenarios,
            concurrency_limit: Arc::new(Semaphore::new(10)), // Limit to 10 concurrent warmings
            shutdown_rx,
        }
    }

    /// Start the predictive warming background task.
    pub async fn start(mut self) {
        // Run every 15 minutes to predict the next arrival window
        let mut ticker = interval(Duration::from_secs(900));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let now = chrono::Utc::now();
                    let next_window_start = (now + Duration::from_secs(900)).hour();
                    
                    info!(
                        hour = next_window_start,
                        minute = ((now.minute() / 15 + 1) * 15) % 60,
                        "Starting 15-minute predictive cache warming cycle"
                    );

                    if let Err(e) = self.warm_next_arrivals(next_window_start).await {
                        warn!(error = %e, "Predictive warming cycle failed");
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Predictive warmer shutting down...");
                    break;
                }
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

        // Fetch both "High Priority" (Whales/Influencers) and "Likely Arrivals"
        let high_priority_users = interaction_repo.get_top_engaged_users(100).await.unwrap_or_default();
        let likely_users = interaction_repo.get_likely_arrivals(target_hour).await?;
        
        info!(
            likely = likely_users.len(),
            priority = high_priority_users.len(),
            target_hour,
            "Identified warming candidates"
        );

        // 2. Trigger warming for each user + scenario
        // Combine and deduplicate
        let mut all_users = likely_users;
        for u in high_priority_users {
            if !all_users.contains(&u) {
                all_users.push(u);
            }
        }

        for user_id in all_users {
            for scenario in &self.warm_scenarios {
                let engine = self.engine.clone();
                let scenario_slug = scenario.clone();
                let permit = self.concurrency_limit.clone().acquire_owned().await;
                
                // Use tokio::spawn to parallelize individual user warming
                tokio::spawn(async move {
                    let _permit = permit; // Hold permit until task finishes
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
