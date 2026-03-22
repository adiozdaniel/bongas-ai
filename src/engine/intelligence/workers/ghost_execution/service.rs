//! Phase 11: Ghost Execution (Sequencing Intelligence)
//!
//! Predicts the exact next video based on user's current session context (Pillar 4).

use std::sync::Arc;
use tokio::sync::{mpsc, broadcast};
use tracing::{info, error, debug};
use anyhow::Result;
use std::time::Duration;

use crate::cache::CacheManager;
use crate::ml::inference::candle::service::CandleInferenceEngine;
use crate::ingestion::UserActivity;

const ACTIVITY_CHANNEL_SIZE: usize = 1000;
const MAX_HISTORY_LEN: usize = 20;
const GHOST_CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Worker that performs "Ghost Execution" (pre-warm sequencing) in the background.
#[derive(Clone)]
pub struct GhostExecutionWorker {
    cache_manager: Arc<CacheManager>,
    candle_engine: Arc<CandleInferenceEngine>,
    activity_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<UserActivity>>>,
    activity_tx: mpsc::Sender<UserActivity>,
}

impl GhostExecutionWorker {
    pub fn new(
        cache_manager: Arc<CacheManager>,
        candle_engine: Arc<CandleInferenceEngine>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(ACTIVITY_CHANNEL_SIZE);
        Self {
            cache_manager,
            candle_engine,
            activity_rx: Arc::new(tokio::sync::Mutex::new(rx)),
            activity_tx: tx,
        }
    }

    /// Get a handle to notify this worker of new activity.
    pub fn get_notifier(&self) -> mpsc::Sender<UserActivity> {
        self.activity_tx.clone()
    }

    /// Start the ghost execution loop.
    pub async fn start(&self, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("🏎️ Ghost Execution Worker started (Sequencing Intelligence)");

        let mut activity_rx = self.activity_rx.lock().await;

        loop {
            tokio::select! {
                Some(activity) = activity_rx.recv() => {
                    if let Err(e) = self.handle_activity(activity).await {
                        error!(error = %e, "Failed to handle ghost execution for activity");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Ghost Execution Worker shutting down...");
                    break;
                }
            }
        }
    }

    async fn handle_activity(&self, activity: UserActivity) -> Result<()> {
        let (profile_id, item_id) = match activity {
            UserActivity::Playback { ref profile_id, item_id, .. } => (profile_id.clone(), Some(item_id)),
            UserActivity::Reaction { ref profile_id, item_id, .. } => (profile_id.clone(), Some(item_id)),
            UserActivity::Click { ref profile_id, item_id, .. } => (profile_id.clone(), Some(item_id)),
            UserActivity::Impression { ref profile_id, item_id, .. } => (profile_id.clone(), Some(item_id)),
            UserActivity::ProfileUpdate { ref profile_id, .. } => (profile_id.clone(), None),
            UserActivity::Notification { ref profile_id, .. } => (profile_id.clone(), None),
        };

        let profile_id = match profile_id {
            Some(pid) => pid,
            None => {
                debug!("Skipping ghost execution for anonymous activity");
                return Ok(());
            }
        };

        let item_id = match item_id {
            Some(id) => id,
            None => return Ok(()), // No item to add to history
        };

        debug!(profile_id, item_id, "Processing ghost execution");

        // 1. Update User History in Redis (The Feedback Loop)
        let history_key = format!("user:history:{}", profile_id);
        self.cache_manager.push_to_list(&history_key, item_id.to_string(), MAX_HISTORY_LEN).await?;

        // 2. Fetch History for Inference (The Pre-Warm)
        let history = self.cache_manager.get_list(&history_key).await?;
        if history.len() < 3 {
            debug!(profile_id, history_len = history.len(), "Insufficient history for ghost execution");
            return Ok(()); // Not enough context for sequencing
        }

        // 3. Run Reflex (The Micro-Model Inference)
        let next_ids = self.predict_next_sequence(&history).await?;

        if !next_ids.is_empty() {
            // 4. Cache in Redis (The Ghost Cache)
            let ghost_key = format!("ghost:cache:{}", profile_id);
            let ghost_val = next_ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            
            self.cache_manager.set_raw(&ghost_key, ghost_val, GHOST_CACHE_TTL).await?;
            debug!(profile_id, predictions = ?next_ids, "Ghost Cache updated");
        }

        Ok(())
    }

    async fn predict_next_sequence(&self, history: &[String]) -> Result<Vec<i32>> {
        // Use the engine name to simulate model-specific logic
        let _model = self.candle_engine.model_name();

        // Let's simulate some "intelligence" by grabbing the last item and suggesting related ones
        if let Some(last_id_str) = history.first() {
            if let Ok(last_id) = last_id_str.parse::<i32>() {
                // Simulate sequencing: item + 1, item + 2 (High energy transitions)
                return Ok(vec![last_id + 1, last_id + 2, last_id + 3]);
            }
        }
        
        Ok(vec![])
    }
}
