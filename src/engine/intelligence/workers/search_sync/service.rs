use std::sync::{Arc, Weak};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug};
use anyhow::Result;
use tantivy::doc;

use crate::engine::coordination::service::BongasEngine;
use crate::search::EmbeddedSearchManager;

/// 💓 Background worker that synchronizes metadata from Postgres to the Embedded Search Index.
pub struct SearchSyncWorker {
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    interval: Duration,
    search_manager: Arc<EmbeddedSearchManager>,
}

impl SearchSyncWorker {
    pub fn new(search_manager: Arc<EmbeddedSearchManager>, interval: Duration) -> Self {
        Self {
            engine: std::sync::Mutex::new(None),
            interval,
            search_manager,
        }
    }

    pub fn set_engine(&self, engine: Weak<BongasEngine>) {
        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(engine);
    }

    /// Start the background search synchronization loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!(
            interval_mins = self.interval.as_secs() / 60,
            "Embedded Search Sync Worker started (M20)"
        );

        // Phase 2.2: Auto-Repair Mode
        if let Err(e) = self.perform_integrity_check().await {
            error!(error = %e, "Search index integrity check failed");
        }

        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_sync_cycle().await {
                        error!(error = %e, "Embedded search sync cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Search Sync Worker shutting down...");
                    break;
                }
            }
        }
    }

    async fn perform_integrity_check(&self) -> Result<()> {
        let searcher = self.search_manager.searcher();
        let num_docs = searcher.num_docs();

        if num_docs == 0 {
            info!("Search index appears empty. Triggering Full Rebuild Mode...");
            self.trigger_full_rebuild().await?;
        }

        Ok(())
    }

    async fn trigger_full_rebuild(&self) -> Result<()> {
        debug!("Starting full search index rebuild...");
        self.run_sync_cycle().await?;
        Ok(())
    }

    async fn run_sync_cycle(&self) -> Result<()> {
        let engine_arc = {
            let guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|w| w.upgrade())
        };

        let engine = match engine_arc {
            Some(e) => e,
            None => return Ok(()),
        };

        let items = engine.execution.item_feature_service.get_items_for_search_sync(500).await?;
        
        if items.is_empty() {
            return Ok(());
        }

        debug!(count = items.len(), "Differential Census: Indexing new/updated content");

        let schema = self.search_manager.schema();

        for item in items {
            // Fix DNA extraction: items.embedding is Option<Vec<f32>>
            let dna_bytes: Vec<u8> = match item.embedding {
                Some(vec) => {
                    vec.iter().flat_map(|f| f.to_le_bytes().to_vec()).collect()
                },
                None => vec![0u8; 128],
            };

            let metadata_json = serde_json::json!({
                "content_type": item.content_type,
                "release_year": item.release_year,
                "popularity": item.popularity_score,
            }).to_string();

            let doc = doc!(
                schema.id => item.item_id as i64,
                schema.title => item.title.unwrap_or_else(|| format!("Item {}", item.item_id)),
                schema.description => item.description.unwrap_or_default(),
                schema.spoken_native => "", 
                schema.vision_dna => dna_bytes,
                schema.metadata => metadata_json
            );

            self.search_manager.upsert_document(doc).await?;
        }

        self.search_manager.commit()?;
        info!("Embedded search index synchronized");

        Ok(())
    }
}
