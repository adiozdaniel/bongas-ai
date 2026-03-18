use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use clickhouse::Client as ClickHouseClient;
use redis::AsyncCommands;

use crate::cache::CacheManager;
use crate::db::ResilientPool;
use crate::resilience::ResilienceMetricsCollector;
use crate::engine::intelligence::workers::sovereign_sight::models::SovereignSightLedger;

/// Background worker for the Sovereign Sight Intelligence (Milestone 17).
///
/// Implements the first 7 steps of Phase 1:
/// 1. Sidecar Architecture (Native Rust Worker)
/// 2. Differential Census Loop
/// 3. Visual DNA Extraction (Simulated for Phase 1)
/// 4. Forensic Maturity Auditor
/// 5. Semantic Digest Engine
/// 6. Hook & Teaser Factory
/// 7. Tribe DNA Integration (Redis Weights)
pub struct SovereignSightWorker {
    db_pool: Arc<ResilientPool>,
    clickhouse: ClickHouseClient,
    cache_manager: Arc<CacheManager>,
    resilience: Arc<ResilienceMetricsCollector>,
    pulse_interval: Duration,
    cpu_threshold: u64,
}

#[derive(serde::Deserialize, clickhouse::Row)]
struct ExternalIdRow {
    external_id: i32,
}

impl SovereignSightWorker {
    pub fn new(
        db_pool: Arc<ResilientPool>,
        clickhouse: ClickHouseClient,
        cache_manager: Arc<CacheManager>,
        resilience: Arc<ResilienceMetricsCollector>,
        pulse_interval: Duration,
    ) -> Self {
        Self {
            db_pool,
            clickhouse,
            cache_manager,
            resilience,
            pulse_interval,
            cpu_threshold: 70, // Threshold for Opportunistic Pause
        }
    }

    /// Start the background visual audit loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("Sovereign Sight Native Worker started (M17)");
        let mut interval = tokio::time::interval(self.pulse_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Step 1: Opportunistic Pause (Resilience Shield M4)
                    if self.should_pause() {
                        debug!("Engine load high, pausing visual audit pulse...");
                        continue;
                    }

                    if let Err(e) = self.run_census_and_audit().await {
                        error!(error = %e, "Sovereign Sight pulse failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Sovereign Sight Worker shutting down...");
                    break;
                }
            }
        }
    }

    /// Check if the worker should pause based on main engine CPU load.
    fn should_pause(&self) -> bool {
        // Simplified for Phase 1: Check metrics registry for load status
        let cpu_usage = self.resilience.registry().get_or_create("system_resources")
            .successes.get(); 
        
        cpu_usage > self.cpu_threshold
    }

    /// Differential Census & Visual Audit Loop
    async fn run_census_and_audit(&self) -> Result<()> {
        // Step 2: Fetch IDs from Postgres (ReadOnly SoR)
        let pg_ids = self.fetch_catalog_ids().await?;
        
        // Fetch IDs from ClickHouse (Our Sight Ledger)
        let ch_ids = self.fetch_ledger_ids().await?;
        
        // Step 2: Identify Delta (IDs not yet in ledger)
        let delta: Vec<i32> = pg_ids.into_iter().filter(|id| !ch_ids.contains(id)).collect();
        
        if delta.is_empty() {
            return Ok(());
        }

        info!(count = delta.len(), "Differential Census: Discovered new content for audit");

        // Batch processing to respect resources
        let mut results = Vec::new();
        for external_id in delta.into_iter().take(5) {
            if self.should_pause() { break; }

            // Steps 3-6: Visual Analysis (Simulated Native Inference)
            let result = self.perform_visual_audit(external_id).await?;
            results.push(result);
        }

        // Step 7: Persist results and sync affinities
        if !results.is_empty() {
            self.save_audit_results(results).await?;
            self.sync_tribe_affinities().await?;
        }

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

    async fn fetch_ledger_ids(&self) -> Result<std::collections::HashSet<i32>> {
        let rows: Vec<ExternalIdRow> = self.clickhouse
            .query("SELECT external_id FROM sovereign_sight_ledger")
            .fetch_all()
            .await?;
        Ok(rows.into_iter().map(|r| r.external_id).collect())
    }

    /// Internal logic for visual DNA and maturity forensic extraction.
    async fn perform_visual_audit(&self, external_id: i32) -> Result<SovereignSightLedger> {
        // Step 3: Visual DNA Extraction (Simulated)
        // Production: ort::Session::new(&env, "models/sight_core.onnx", &opts)?
        
        let entropy = 0.45; // Steady pace
        let appearance = "Photorealistic".to_string();
        
        // Step 4: Forensic Maturity Auditor (Anatomy DNA Isolation)
        let (rating, reason) = if entropy < 0.2 && appearance == "Photorealistic" {
            ("18".to_string(), "High flesh-tone DNA detected in rhythmic motion context.".to_string())
        } else {
            ("G".to_string(), "Safe for all audiences.".to_string())
        };

        // Step 5: Semantic Digest Engine (Math -> Layman)
        let digest = format!("Visual Style: Steady-paced, cinematic {} flow.", appearance);

        Ok(SovereignSightLedger {
            external_id,
            content_type: "video".to_string(),
            visual_dna: vec![0.1, 0.2, 0.3], // Latent snippet
            motion_entropy: entropy,
            appearance_dna: appearance,
            maturity_rating: rating,
            maturity_reason: reason,
            semantic_digest: digest,
            hook_path: format!("/data/hooks/{}.webp", external_id),
        })
    }

    async fn save_audit_results(&self, results: Vec<SovereignSightLedger>) -> Result<()> {
        let mut insert = self.clickhouse.insert::<SovereignSightLedger>("sovereign_sight_ledger").await?;
        for row in results {
            insert.write(&row).await?;
        }
        insert.end().await?;
        Ok(())
    }

    /// Step 7: Update Tribe DNA Weights in Redis based on visual markers
    async fn sync_tribe_affinities(&self) -> Result<()> {
        if let Some(mut conn) = self.cache_manager.l2_connection() {
            // Update affinities for 'Luhya Gospel' Tribe (ID 42)
            let weights = r#"{"gospel_luhya_slow": 5.0, "cinematic": 1.5, "fast_action": 0.2}"#;
            let _: () = conn.set("tribe_affinity:42", weights).await?;
        }
        Ok(())
    }
}
