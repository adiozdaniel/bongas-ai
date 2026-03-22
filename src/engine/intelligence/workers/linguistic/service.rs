//! Phase 3.2: Linguistic Intelligence (The Linguist)
//!
//! Transforms raw transcripts into semantic, multi-lingual metadata.
//! Handles language detection, internet-augmented dialect mapping, and fan-out.

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{info, error, debug};
use clickhouse::Client as ClickHouseClient;
use tantivy::doc;

use crate::db::ResilientPool;
use crate::search::EmbeddedSearchManager;
use crate::engine::intelligence::ai::hive_mind::service::HiveMindConnector;
use crate::engine::intelligence::workers::sound_listener::models::{AudioTranscript, ProcessedAudioIntelligence};

/// 🗣️ Background worker for multi-lingual linguistic mapping.
pub struct LinguisticWorker {
    clickhouse: Arc<ClickHouseClient>,
    search_manager: Arc<EmbeddedSearchManager>,
    hive_mind: Arc<HiveMindConnector>,
    interval: Duration,
}

impl LinguisticWorker {
    pub fn new(
        _db_pool: Arc<ResilientPool>,
        clickhouse: Arc<ClickHouseClient>,
        search_manager: Arc<EmbeddedSearchManager>,
        hive_mind: Arc<HiveMindConnector>,
        interval: Duration,
    ) -> Self {
        Self {
            clickhouse,
            search_manager,
            hive_mind,
            interval,
        }
    }

    /// Start the linguistic processing loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("🗣️ Linguistic Worker started (M20 Phase 3)");
        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.run_linguistic_cycle().await {
                        error!(error = %e, "Linguistic cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Linguistic Worker shutting down...");
                    break;
                }
            }
        }
    }

    async fn run_linguistic_cycle(&self) -> Result<()> {
        // 1. Fetch unprocessed transcripts from ClickHouse
        let transcripts = self.fetch_unprocessed_transcripts().await?;
        
        if transcripts.is_empty() {
            return Ok(());
        }

        info!(count = transcripts.len(), "Linguistic Hub: Processing new transcripts");

        for transcript in transcripts {
            self.process_transcript(transcript).await?;
        }

        Ok(())
    }

    async fn fetch_unprocessed_transcripts(&self) -> Result<Vec<AudioTranscript>> {
        // Use a subquery check to avoid re-processing records that already exist in the processed table.
        // This is more reliable than ALTER TABLE ... UPDATE in high-throughput ClickHouse environments.
        let query = r#"
            SELECT * FROM audio_transcripts 
            WHERE item_id NOT IN (SELECT item_id FROM processed_audio_intelligence)
            LIMIT 10
        "#;
        let rows = self.clickhouse.query(query).fetch_all::<AudioTranscript>().await?;
        Ok(rows)
    }

    async fn process_transcript(&self, raw: AudioTranscript) -> Result<()> {
        debug!(item_id = raw.item_id, "Linguistic Analysis: Mapping dialects and translating...");

        // 2. Language ID & Dialect Mapping (Stage 2)
        // If language is unknown or a complex dialect, use HiveMind (Internet Augmented)
        let (language_id, english_translation, mapping) = self.resolve_linguistic_metadata(&raw).await?;

        // 3. Save Processed Intelligence
        let processed = ProcessedAudioIntelligence {
            item_id: raw.item_id,
            native_text: raw.transcript.clone(),
            english_translation: english_translation.clone(),
            language_id: language_id.clone(),
            dialect_mappings: serde_json::to_string(&mapping)?,
            processed_at: chrono::Utc::now(),
        };

        self.save_processed_intelligence(processed).await?;

        // 4. Fan-Out: Stage 3 (Distribution)
        // Update Search Index with Dual-Mapping (Native + Translation)
        self.update_search_index(raw.item_id, &raw.transcript, &english_translation).await?;

        Ok(())
    }

    async fn resolve_linguistic_metadata(&self, raw: &AudioTranscript) -> Result<(String, String, std::collections::HashMap<String, String>)> {
        // In production, this calls the Swahili Brain (SLM) or HiveMind
        // for deep linguistic mapping.
        
        let text = &raw.transcript;
        let mut mapping = std::collections::HashMap::new();
        let mut language_id = raw.detected_language.clone();
        let mut translation = text.clone();

        // Example Dialect Mapping Logic
        if text.contains("maze") || text.contains("inabamba") {
            language_id = "sheng".to_string();
            mapping.insert("maze".into(), "surely/man".into());
            mapping.insert("inabamba".into(), "is amazing/cool".into());
            translation = text.replace("maze", "surely").replace("inabamba", "is amazing");
        } else if language_id != "en" {
            // Trigger HiveMind for Internet-Augmented translation if not English
            debug!(item_id = raw.item_id, lang = %language_id, "Linguistic Hub: Triggering Internet-Augmented translation");
            
            // Production Call: Execute dialect mapping via the HiveMind Bridge
            translation = self.hive_mind.translate(&raw.transcript, &language_id).await
                .unwrap_or_else(|_| format!("[Fallback Translation] {}", text));
        }

        Ok((language_id, translation, mapping))
    }

    async fn save_processed_intelligence(&self, data: ProcessedAudioIntelligence) -> Result<()> {
        let mut insert = self.clickhouse.insert::<ProcessedAudioIntelligence>("processed_audio_intelligence").await?;
        insert.write(&data).await?;
        insert.end().await?;
        Ok(())
    }

    async fn update_search_index(&self, item_id: i32, native: &str, translated: &str) -> Result<()> {
        let schema = self.search_manager.schema();
        // Delete old entry first
        self.search_manager.delete_item(item_id as i64)?;

        // Index both fields for dual-language discovery
        let doc = doc!(
            schema.id => item_id as i64,
            schema.spoken_native => native.to_string(),
            schema.spoken_translated => translated.to_string(),
        );

        self.search_manager.upsert_document(doc).await?;
        self.search_manager.commit()?;
        
        debug!(item_id, "Search index updated with granular spoken content");
        Ok(())
    }
}
