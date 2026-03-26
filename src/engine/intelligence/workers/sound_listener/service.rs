//! Phase 3.1: Sound Listener Intelligence (The Ear)
//! 
//! Exclusive focus on raw audio-to-text transcription using pure-Rust Whisper.
//! Acts as the primary data generator for the downstream Linguistic Worker.

use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug, warn};
use anyhow::Result;
use clickhouse::Client as ClickHouseClient;
use std::sync::{Arc, Weak};

use crate::engine::coordination::service::BongasEngine;
use crate::db::ResilientPool;
use crate::engine::intelligence::forensics::models::AudioTranscript;
// Removed Whisper specific imports to uphold The Sovereign Paradigm

/// 👂 The Ear: Sovereign DNA Extractor for Audio Intelligence.
/// 
/// Instead of relying on external, generalist models (like Whisper), 
/// this worker implements the "Sight-Core as the Ultimate Compressor" principle.
/// It extracts high-dimensional Audio DNA from the frozen foundation backbone,
/// ensuring that all intelligence is distilled from the project's own compute budget.
pub struct SoundListenerWorker {
    engine: std::sync::Mutex<Option<Weak<BongasEngine>>>,
    db_pool: Arc<ResilientPool>,
    interval: Duration,
    // Resource awareness threshold: The Glass Jar constraint
    cpu_threshold: u64,
}

impl SoundListenerWorker {
    pub fn new(
        db_pool: Arc<ResilientPool>,
        _clickhouse: Option<Arc<ClickHouseClient>>,
        interval: Duration,
    ) -> Self {
        Self {
            engine: std::sync::Mutex::new(None),
            db_pool,
            interval,
            cpu_threshold: 60, 
        }
    }

    pub fn set_engine(&self, engine: Weak<BongasEngine>) {
        let mut guard = self.engine.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(engine);
    }

    /// Start the sound listener loop.
    pub async fn start(self: Arc<Self>, mut shutdown_rx: broadcast::Receiver<()>) {
        info!("👂 Sound Listener (The Ear) active: Operating under The Sovereign Paradigm");
        
        let mut ticker = interval(self.interval);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if self.should_yield().await {
                        debug!("Sight-Core under heavy load: The Ear is yielding resources (Glass Jar Constraint)...");
                        continue;
                    }

                    if let Err(e) = self.run_listening_cycle().await {
                        error!(error = %e, "Sound listener cycle failed");
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Sound Listener Worker shutting down...");
                    break;
                }
            }
        }
    }

    /// Distillation over Memorization: Extracting semantic DNA from raw audio.
    async fn run_listening_cycle(&self) -> Result<()> {
        let items = self.fetch_pending_audio_items().await?;
        if items.is_empty() {
            return Ok(());
        }

        info!(count = items.len(), "The Ear: Extracting Sovereign Audio DNA from raw signal");

        for item_id in items {
            // 2. DNA Extraction using the Frozen Sight-Core Backbone
            let audio_dna = self.extract_audio_dna(item_id).await?;
            
            // 3. Save DNA record to ClickHouse for downstream Student Head decoding
            self.save_audio_dna(item_id, audio_dna).await?;
        }

        Ok(())
    }

    async fn fetch_pending_audio_items(&self) -> Result<Vec<i32>> {
        let res = self.db_pool.execute(|pool| async move {
            let rows: Vec<(i32,)> = sqlx::query_as("SELECT item_id FROM bongas.item_features WHERE is_active = true LIMIT 5")
                .fetch_all(&pool)
                .await?;
            Ok(rows.into_iter().map(|r| r.0).collect())
        }).await;

        match res {
            Ok(ids) => Ok(ids),
            Err(e) => {
                warn!(error = %e, "Failed to fetch pending audio items");
                Ok(vec![])
            }
        }
    }

    /// Sovereign DNA Extraction: Mapping raw PCM to the Sight-Core latent space.
    async fn extract_audio_dna(&self, item_id: i32) -> Result<Vec<f32>> {
        debug!(item_id, "The Ear: Running Sight-Core native DNA extraction...");
        
        let engine_weak = self.engine.lock().unwrap().clone()
            .ok_or_else(|| anyhow::anyhow!("Engine reference missing"))?;
        let engine = engine_weak.upgrade()
            .ok_or_else(|| anyhow::anyhow!("Engine already dropped"))?;

        // 4. Physical Audio Decoding
        let content_path = format!("data/hooks/{}.mp4", item_id);
        let pcm = if std::path::Path::new(&content_path).exists() {
            crate::engine::intelligence::workers::sound_listener::audio::decode_to_pcm(&content_path)?
        } else {
            // Fallback for missing files: Distillation requires signal.
            return Err(anyhow::anyhow!("Source content missing for item {}", item_id));
        };

        // 5. Execute DNA Extraction (The "Sight-Core as the Ultimate Compressor" phase)
        // In this architecture, we treat the Sight-Core as a multi-modal encoder.
        // We pass the PCM signal to the embeddings manager to generate the dense DNA vector.
        let audio_dna = engine.ml_pillar.inference.embeddings.generate_audio_dna(&pcm).await?;
        
        Ok(audio_dna)
    }

    async fn save_audio_dna(&self, item_id: i32, dna: Vec<f32>) -> Result<()> {
        let engine_weak = self.engine.lock().unwrap().clone()
            .ok_or_else(|| anyhow::anyhow!("Engine reference missing"))?;
        let engine = engine_weak.upgrade()
            .ok_or_else(|| anyhow::anyhow!("Engine already dropped"))?;

        debug!(item_id, "The Ear: Syncing Sovereign Audio DNA via ForensicPillar");
        
        let row = AudioTranscript {
            item_id,
            transcript: "".to_string(), // Text is now distilled by the Student Language Head
            audio_dna: Some(dna),
            detected_language: "dna_embedded".to_string(),
            extracted_at: chrono::Utc::now(),
            processed: false,
        };

        engine.intelligence.forensics.record_audio_transcript(row).await?;
        
        Ok(())
    }

    async fn should_yield(&self) -> bool {
        let mut sys = sysinfo::System::new();
        sys.refresh_cpu_usage();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        sys.refresh_cpu_usage();
        let global_usage = sys.global_cpu_usage();
        global_usage > self.cpu_threshold as f32
    }
}
