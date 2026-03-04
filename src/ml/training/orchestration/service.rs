//! Phase 15: Training Orchestrator (The Ghost Scheduler)
//!
//! Orchestrates the "One-Shot Harvest" strategy for client-binary deployments.
//!
//! 1. Detects first launch (no model present).
//! 2. Extracts historical sequences from ClickHouse with persona/device context.
//! 3. Streams the "Master Snapshot" to the central training server with Zstd compression.
//! 4. Polls for the resulting ONNX model.

use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::{info, error, debug};
use clickhouse::Client as ClickHouseClient;
use reqwest::Client as HttpClient;
use async_compression::tokio::write::ZstdEncoder;
use tokio::io::AsyncWriteExt;

use crate::config::MlConfig;
use crate::security::SecurityManager;
use crate::ml::model_loader::ModelLoader;

/// Represents a single interaction sequence for training.
#[derive(Debug, Serialize, Deserialize, clickhouse::Row)]
pub struct HarvestedInteraction {
    pub user_id: i32,
    pub item_id: i32,
    pub interaction_type: String,
    pub device_type: String,
    pub profile_id: String,
    pub maturity_rating: String,
    pub genre: String,
    pub watch_duration_seconds: i32,
    pub created_at: u64,
}

/// Orchestrates the data harvest and training loop.
pub struct TrainingOrchestrator {
    config: MlConfig,
    clickhouse: ClickHouseClient,
    http_client: HttpClient,
    security_manager: Arc<SecurityManager>,
    model_loader: Arc<ModelLoader>,
}

impl TrainingOrchestrator {
    pub fn new(
        config: MlConfig,
        clickhouse: ClickHouseClient,
        security_manager: Arc<SecurityManager>,
        model_loader: Arc<ModelLoader>,
    ) -> Self {
        Self {
            config,
            clickhouse,
            http_client: HttpClient::builder()
                .timeout(Duration::from_secs(600)) // 10 min timeout for streaming
                .build()
                .unwrap_or_default(),
            security_manager,
            model_loader,
        }
    }

    /// Run the one-shot harvest if no model is present.
    pub async fn run_one_shot_harvest(&self) -> Result<()> {
        // 1. Check if we already have models loaded
        if self.model_loader.loaded_count().await > 0 {
            debug!("Models already present, skipping one-shot harvest");
            return Ok(());
        }

        info!("Starting Streaming One-Shot Harvest for first-launch training...");

        // 2. Perform streaming upload
        self.stream_harvest_to_server().await?;

        info!("One-Shot Harvest stream completed successfully. Waiting for model training...");
        
        Ok(())
    }

    /// Stream data from ClickHouse to central server with Zstd compression.
    async fn stream_harvest_to_server(&self) -> Result<()> {
        let url = format!("{}/api/v1/training/harvest", self.config.central_server_url);
        
        // hardware validation
        let hardware_id = self.security_manager.get_hardware_id().await.unwrap_or_default();
        if !self.security_manager.is_validated().await {
            return Err(anyhow::anyhow!("Cannot upload harvest: License not validated"));
        }

        let (writer, reader) = tokio::io::duplex(64 * 1024); // 64KB buffer
        
        // ClickHouse Query
        let query = r#"
            SELECT 
                user_id, item_id, interaction_type, 
                'unknown' as device_type, 'default' as profile_id, 'GE' as maturity_rating, 'unknown' as genre,
                watch_duration_seconds, created_at
            FROM user_interactions
            WHERE created_at > (toUnixTimestamp(now()) - 7776000)
            ORDER BY user_id, created_at ASC
        "#;

        let mut cursor = self.clickhouse.query(query).fetch::<HarvestedInteraction>()?;

        // Background task to pump data into the encoder
        // Fix #L2: Properly handle errors in background task
        let handle = tokio::spawn(async move {
            let mut encoder = ZstdEncoder::new(writer);
            while let Ok(Some(row)) = cursor.next().await {
                let json_row = serde_json::to_vec(&row)?;
                encoder.write_all(&json_row).await?;
                encoder.write_all(b"\n").await?; // NDJSON format
            }
            encoder.shutdown().await?;
            Ok::<(), anyhow::Error>(())
        });

        // Use the reader as the request body
        let stream = tokio_util::io::ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);

        let response = self.http_client
            .post(&url)
            .header("X-Hardware-ID", hardware_id)
            .header("Content-Encoding", "zstd")
            .header("Content-Type", "application/x-ndjson")
            .body(body)
            .send()
            .await
            .context("Failed to stream harvest to training server")?;

        // Ensure the background task completed successfully
        if let Err(e) = handle.await? {
            error!(error = %e, "Background data harvest task failed");
            return Err(e);
        }

        if !response.status().is_success() {
            let status = response.status();
            error!(status = %status, "Harvest stream upload failed");
            return Err(anyhow::anyhow!("Training server rejected stream: {}", status));
        }

        Ok(())
    }
}
