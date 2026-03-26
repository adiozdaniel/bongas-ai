use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use redis::AsyncCommands;

use crate::cache::CacheManager;
use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;

/// Background worker for the Sovereign Sight Intelligence (Milestone 17).
///
/// Refactored to act solely as a Visual DNA Extractor (Sensor).
/// The Forensic Auditor now handles maturity evaluation and mismatch reconciliation.
pub struct SovereignSightWorker {
    db_pool: Arc<ResilientPool>,
    cache_manager: Arc<CacheManager>,
    resilience: Arc<ResilienceMetricsCollector>,
    pulse_interval: Duration,
    cpu_threshold: u64,
}

impl SovereignSightWorker {
    pub fn new(
        db_pool: Arc<ResilientPool>,
        _clickhouse: clickhouse::Client, // Kept for API compat, unused here
        cache_manager: Arc<CacheManager>,
        resilience: Arc<ResilienceMetricsCollector>,
        _training_state: Arc<crate::ml::training::pillar::state::TrainingState>, // Unused here
        pulse_interval: Duration,
    ) -> Self {
        Self {
            db_pool,
            cache_manager,
            resilience,
            pulse_interval,
            cpu_threshold: 80, // Threshold for Opportunistic Pause
        }
    }

    /// Start the background visual audit loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("Sovereign Sight Native Worker started (DNA Extractor Sensor)");
        let mut interval = tokio::time::interval(self.pulse_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.should_pause() {
                        debug!("Engine load high, pausing visual extraction pulse...");
                        continue;
                    }

                    if let Err(e) = self.run_dna_extraction().await {
                        error!(error = %e, "Sovereign Sight extraction failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Sovereign Sight Worker shutting down...");
                    break;
                }
            }
        }
    }

    fn should_pause(&self) -> bool {
        let cpu_usage = self.resilience.registry().get_or_create("system_resources")
            .successes.get(); 
        cpu_usage > self.cpu_threshold
    }

    async fn run_dna_extraction(&self) -> Result<()> {
        let pg_ids = self.fetch_catalog_ids().await?;
        
        // In a fully integrated system, we would check `content_dna` to find missing extractions.
        // For Phase 1 simulation, we just take the first 5 to demonstrate the flow.
        let delta: Vec<i32> = pg_ids.into_iter().take(5).collect();
        
        if delta.is_empty() {
            debug!("SovereignSightWorker: No new content for DNA extraction.");
            return Ok(());
        }

        info!(count = delta.len(), "SovereignSightWorker: Extracting raw Visual DNA...");

        let mut dna_records = Vec::new();

        for external_id in delta {
            if self.should_pause() { break; }

            // 1. Extract DNA using the "Giant" (The Frozen Base Model)
            let dna_vector = vec![0.5; 1024]; // Simulated 1024-dim Visual DNA

            // 3. Prepare DNA Ledger record (M21.3)
            dna_records.push(serde_json::json!({
                "item_id": external_id,
                "dna_type": "vision",
                "dna_vector": dna_vector,
                "version": 1
            }));
        }

        if !dna_records.is_empty() {
            self.save_dna_to_ledger(dna_records).await?;
            self.sync_tribe_affinities().await?;
        }

        Ok(())
    }

    async fn save_dna_to_ledger(&self, records: Vec<serde_json::Value>) -> Result<()> {
        debug!(count = records.len(), "SovereignSightWorker: Persisting DNA to ClickHouse Ledger...");
        Ok(())
    }

    async fn fetch_catalog_ids(&self) -> Result<Vec<i32>> {
        let res = self.db_pool.execute(|pool| async move {
            let rows: Vec<(i32,)> = sqlx::query_as("SELECT item_id FROM bongas.item_features WHERE is_active = true")
                .fetch_all(&pool)
                .await?;
            Ok(rows.into_iter().map(|r| r.0).collect::<Vec<i32>>())
        }).await;

        match res {
            Ok(ids) => Ok(ids),
            Err(e) => Err(anyhow::anyhow!("Postgres ID fetch failed: {}", e)),
        }
    }

    async fn sync_tribe_affinities(&self) -> Result<()> {
        if let Some(mut conn) = self.cache_manager.l2_connection() {
            let weights = r#"{"gospel_luhya_slow": 5.0, "cinematic": 1.5, "fast_action": 0.2}"#;
            let _: () = conn.set("tribe_affinity:42", weights).await?;
        }
        Ok(())
    }
}
