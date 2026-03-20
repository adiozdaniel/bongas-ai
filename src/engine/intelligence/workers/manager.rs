//! Engine pulse workers and background maintenance tasks.

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};
use crate::engine::coordination::service::BongasEngine;
use crate::engine::intelligence::workers::tribe_orchestrator::service::TribeOrchestrator;
use crate::engine::intelligence::workers::regional_pulse::service::RegionalPulseWorker;
use crate::engine::intelligence::workers::fatigue_sync::service::FatigueSynchronizer;
use crate::engine::intelligence::workers::reasoning::service::ReasoningWorker;
use crate::engine::intelligence::workers::digest_worker::service::DigestWorker;
use crate::engine::intelligence::workers::search_sync::service::SearchSyncWorker;
use crate::engine::intelligence::workers::signal_decay::service::SignalDecayWorker;
use crate::engine::intelligence::workers::sovereign_sight::service::SovereignSightWorker;
use crate::engine::intelligence::workers::ghost_execution::service::GhostExecutionWorker;
use crate::engine::intelligence::workers::sound_listener::service::SoundListenerWorker;

/// 💓 Workers: Background maintenance and task orchestration.
pub struct WorkersManager {
    tribe_orchestrator: Option<Arc<TribeOrchestrator>>,
    regional_pulse: Option<Arc<RegionalPulseWorker>>,
    fatigue_sync: Option<Arc<FatigueSynchronizer>>,
    reasoning: Option<Arc<ReasoningWorker>>,
    digest: Option<Arc<DigestWorker>>,
    search_sync: Option<Arc<SearchSyncWorker>>,
    signal_decay: Option<Arc<SignalDecayWorker>>,
    sovereign_sight: Option<Arc<SovereignSightWorker>>,
    ghost_execution: Option<Arc<GhostExecutionWorker>>,
    sound_listener: Option<Arc<SoundListenerWorker>>,
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
            reasoning: None,
            digest: None,
            search_sync: None,
            signal_decay: None,
            sovereign_sight: None,
            ghost_execution: None,
            sound_listener: None,
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

    pub fn with_reasoning(mut self, worker: Arc<ReasoningWorker>) -> Self {
        self.reasoning = Some(worker);
        self
    }

    pub fn with_digest(mut self, worker: Arc<DigestWorker>) -> Self {
        self.digest = Some(worker);
        self
    }

    pub fn with_search_sync(mut self, worker: Arc<SearchSyncWorker>) -> Self {
        self.search_sync = Some(worker);
        self
    }

    pub fn with_signal_decay(mut self, worker: Arc<SignalDecayWorker>) -> Self {
        self.signal_decay = Some(worker);
        self
    }

    pub fn with_sovereign_sight(mut self, worker: Arc<SovereignSightWorker>) -> Self {
        self.sovereign_sight = Some(worker);
        self
    }

    pub fn with_ghost_execution(mut self, worker: Arc<GhostExecutionWorker>) -> Self {
        self.ghost_execution = Some(worker);
        self
    }

    pub fn with_sound_listener(mut self, worker: Arc<SoundListenerWorker>) -> Self {
        self.sound_listener = Some(worker);
        self
    }

    /// Propagate engine reference to workers that need it (like DigestWorker).
    pub fn set_engine(&self, engine: std::sync::Weak<BongasEngine>) {
        if let Some(ref worker) = self.digest {
            worker.set_engine(engine.clone());
        }
        if let Some(ref worker) = self.search_sync {
            worker.set_engine(engine.clone());
        }
        if let Some(ref worker) = self.sound_listener {
            worker.set_engine(engine);
        }
    }

    /// Notify the ghost execution worker of a user event.
    pub fn notify_ghost_execution(&self, activity: crate::ingestion::UserActivity) {
        if let Some(ref worker) = self.ghost_execution {
            let tx = worker.get_notifier();
            let _ = tx.try_send(activity);
        }
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

        // 4. Start Reasoning Worker
        if let Some(ref worker) = self.reasoning {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 5. Start Digest Worker
        if let Some(ref worker) = self.digest {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 6. Start Search Sync Worker
        if let Some(ref worker) = self.search_sync {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 7. Start Signal Decay Worker
        if let Some(ref worker) = self.signal_decay {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 8. Start Sovereign Sight Worker (Milestone 17)
        if let Some(ref worker) = self.sovereign_sight {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 9. Start Ghost Execution Worker (Pillar 4)
        if let Some(ref worker) = self.ghost_execution {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
            });
        }

        // 10. Start Sound Listener Worker (M20 Phase 3)
        if let Some(ref worker) = self.sound_listener {
            let worker = worker.clone();
            let shutdown_rx = shutdown_tx.subscribe();
            tokio::spawn(async move {
                worker.start(shutdown_rx).await;
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
                FROM bongas.user_interactions
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
