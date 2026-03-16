use std::sync::{Arc, Weak};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;
use meilisearch_sdk::client::Client;

use crate::engine::coordination::service::BongasEngine;

/// 🔍 Search Document: Optimized for Meilisearch keyword indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub item_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub creators: Vec<String>,
    pub release_year: Option<i32>,
    pub content_type: Option<String>,
    pub popularity_score: f32,
}

pub struct SearchSyncWorker {
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    interval: Duration,
    client: Client,
    index_name: String,
}

impl SearchSyncWorker {
    pub fn new(host: String, api_key: String, index_name: String, interval: Duration) -> Self {
        Self {
            engine: std::sync::Mutex::new(None),
            interval,
            client: Client::new(host, Some(api_key)).expect("Meilisearch client init failed"),
            index_name,
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
            index = %self.index_name,
            "Search Sync Worker started"
        );

        // Ensure index exists and is configured
        if let Err(e) = self.setup_index().await {
            error!(error = %e, "Failed to setup Meilisearch index");
        }

        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_sync_cycle().await {
                        error!(error = %e, "Search sync cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Search Sync Worker shutting down...");
                    break;
                }
            }
        }
    }

    async fn setup_index(&self) -> Result<()> {
        let index = self.client.index(&self.index_name);
        
        // Configure ranking rules and searchable attributes
        // In a real environment, we'd only do this once
        index.set_searchable_attributes(["title", "description", "genres", "tags", "creators"]).await?;
        index.set_filterable_attributes(["genres", "content_type", "release_year"]).await?;
        index.set_ranking_rules([
            "words",
            "typo",
            "proximity",
            "attribute",
            "sort",
            "exactness",
            "popularity_score:desc"
        ]).await?;

        Ok(())
    }

    async fn run_sync_cycle(&self) -> Result<()> {
        debug!("Running search synchronization cycle...");

        let engine_arc = {
            let guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|w| w.upgrade())
        };

        let engine = match engine_arc {
            Some(e) => e,
            None => return Ok(()),
        };

        // 1. Fetch recently updated items from DB
        // For Milestone 10, we'll fetch a batch of active items.
        // In production, we'd use a 'last_sync_at' cursor or CDC.
        let items = engine.execution.item_feature_service.get_items_for_search_sync(100).await?;
        
        if items.is_empty() {
            return Ok(());
        }

        // 2. Map to SearchDocument
        let documents: Vec<SearchDocument> = items.into_iter().map(|item| {
            let genres = match item.genres {
                Some(JsonValue::Array(arr)) => arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                _ => vec![],
            };
            let tags = match item.tags {
                Some(JsonValue::Array(arr)) => arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                _ => vec![],
            };
            let creators = match item.creators {
                Some(JsonValue::Array(arr)) => arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                _ => vec![],
            };

            SearchDocument {
                id: item.item_id.to_string(),
                item_id: item.item_id,
                title: item.title.unwrap_or_else(|| format!("Item {}", item.item_id)),
                description: item.description,
                genres,
                tags,
                creators,
                release_year: item.release_year,
                content_type: item.content_type,
                popularity_score: item.popularity_score.unwrap_or(0.0),
            }
        }).collect();

        // 3. Push to Meilisearch
        let index = self.client.index(&self.index_name);
        index.add_documents(&documents, Some("id")).await?;

        info!(count = documents.len(), "Synchronized items to Meilisearch");

        Ok(())
    }
}
