//! Engine pulse workers and background maintenance tasks.

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use crate::engine::coordination::service::BongasEngine;
use crate::engine::intelligence::workers::tribe_orchestrator::service::TribeOrchestrator;
use crate::engine::intelligence::workers::regional_pulse::service::RegionalPulseWorker;
use crate::engine::intelligence::workers::fatigue_sync::service::FatigueSynchronizer;

/// 💓 Workers: Background maintenance and task orchestration.
pub struct WorkersManager {
    tribe_orchestrator: Option<Arc<TribeOrchestrator>>,
    regional_pulse: Option<Arc<RegionalPulseWorker>>,
    fatigue_sync: Option<Arc<FatigueSynchronizer>>,
}

impl Default for WorkersManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkersManager {
    pub fn new() -> Self {
        Self {
            tribe_orchestrator: None,
            regional_pulse: None,
            fatigue_sync: None,
        }
    }

    pub fn with_tribe_orchestrator(mut self, orchestrator: Arc<TribeOrchestrator>) -> Self {
        self.tribe_orchestrator = Some(orchestrator);
        self
    }

    pub fn with_regional_pulse(mut self, worker: Arc<RegionalPulseWorker>) -> Self {
        self.regional_pulse = Some(worker);
        self
    }

    pub fn with_fatigue_sync(mut self, sync: Arc<FatigueSynchronizer>) -> Self {
        self.fatigue_sync = Some(sync);
        self
    }

    /// Start all managed background workers.
    pub async fn start(&self, shutdown_tx: broadcast::Sender<()>) {
        info!("💓 Starting background workers...");

        // 1. Start Tribe Orchestrator
        if let Some(ref orchestrator) = self.tribe_orchestrator {
            let orchestrator = orchestrator.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                orchestrator.start(shutdown_rx).await;
            });
        }

        // 2. Start Regional Pulse Worker
        if let Some(ref worker) = self.regional_pulse {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 3. Start Fatigue Synchronizer
        if let Some(ref sync) = self.fatigue_sync {
            let sync = sync.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                sync.start(shutdown_rx).await;
            });
        }
    }
}

impl BongasEngine {
    /// Identify low-performing scenarios based on ClickThrough Rate (CTR) from ClickHouse.
    pub async fn get_low_performing_scenarios(&self) -> Vec<String> {
        if let Some(ref ch) = self.intelligence.clickhouse_client() {
            info!("Querying ClickHouse for scenario performance...");
            
            let query = r#"
                SELECT 
                    scenario_slug,
                    countIf(interaction_type = 'click') / countIf(interaction_type = 'impression') as ctr
                FROM user_interactions
                WHERE created_at > (toUnixTimestamp(now()) - 86400)
                GROUP BY scenario_slug
                HAVING ctr < 0.02
                ORDER BY ctr ASC
            "#;

            let results: Vec<(String, f64)> = match ch.query(query).fetch_all().await {
                Ok(res) => res,
                Err(e) => {
                    warn!(error = %e, "Failed to fetch low-performing scenarios from ClickHouse");
                    return vec![];
                }
            };

            results.into_iter().map(|(slug, _)| slug).collect()
        } else {
            vec![]
        }
    }

    /// Start cache warming background task
    pub fn start_cache_warming(self: Arc<Self>, warm_scenarios: Vec<String>, interval: std::time::Duration) {
        let scenarios_clone = warm_scenarios.clone();
        let cache_manager = self.cache.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        
        let cache_warmer = Arc::new(crate::cache::warming::service::CacheWarmer::new(
            cache_manager,
            scenarios_clone,
            interval,
            shutdown_rx,
        ));

        tokio::spawn(async move {
            cache_warmer.start().await;
        });

        info!(
            scenarios = ?warm_scenarios,
            interval_secs = interval.as_secs(),
            "Cache warming started"
        );
    }
}
