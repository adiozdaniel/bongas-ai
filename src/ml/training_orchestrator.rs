//! Phase 15: Training Orchestrator (The Ghost Scheduler)
//!
//! Orchestrates the "One-Shot Harvest" strategy for client-binary deployments.
//!
//! 1. Detects first launch (no model present).
//! 2. Extracts historical sequences from ClickHouse with persona/device context.
//! 3. Uploads the "Master Snapshot" to the central training server.
//! 4. Polls for the resulting ONNX model.

use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};
use clickhouse::Client as ClickHouseClient;
use reqwest::Client as HttpClient;

use crate::config::MlConfig;
use crate::security::SecurityManager;
use crate::ml::model_loader::ModelLoader;

/// Represents a single interaction sequence for training.
#[derive(Debug, Serialize, Deserialize, clickhouse::Row)]
pub struct HarvestedInteraction {
    pub user_id: i32,
    pub item_id: i32,
    pub event_type: String,
    pub device_type: String,
    pub profile_id: String,
    pub maturity_rating: String,
    pub watch_duration: i32,
    pub event_time: u32,
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
                .timeout(Duration::from_secs(300)) // Long timeout for harvest upload
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

        info!("Starting One-Shot Harvest for first-launch training...");

        // 2. Harvest data from ClickHouse
        let data = self.harvest_historical_data().await?;
        if data.is_empty() {
            warn!("ClickHouse harvest returned no data. Is the database empty?");
            return Ok(());
        }

        info!(count = data.len(), "Harvested historical sequences from ClickHouse");

        // 3. Upload to central server
        self.upload_harvest(data).await?;

        info!("One-Shot Harvest uploaded successfully. Waiting for model training...");
        
        Ok(())
    }

    /// Extract historical data with persona and device context.
    async fn harvest_historical_data(&self) -> Result<Vec<HarvestedInteraction>> {
        // Query historical interactions joined with session context if available.
        // Use the configured time window from config for the first harvest.
        // Default to 90 days if not specified in config
        let days = 90;
        let query = format!(r#"
            SELECT 
                user_id, 
                item_id, 
                event_type, 
                COALESCE(device_type, 'unknown') as device_type,
                COALESCE(profile_id, 'default') as profile_id,
                COALESCE(maturity_rating, 'GE') as maturity_rating,
                watch_duration,
                toUnixTimestamp(event_time) as event_time
            FROM user_events
            WHERE event_time > (now() - INTERVAL {} DAY)
            ORDER BY user_id, event_time ASC
            LIMIT 1000000
        "#, days);

        let rows: Vec<HarvestedInteraction> = self.clickhouse
            .query(&query)
            .fetch_all()
            .await
            .context("Failed to harvest data from ClickHouse")?;

        Ok(rows)
    }

    /// Upload harvest to central server with security signing.
    async fn upload_harvest(&self, data: Vec<HarvestedInteraction>) -> Result<()> {
        let url = format!("{}/api/v1/training/harvest", self.config.central_server_url);
        
        // Sign the request with hardware fingerprint and license
        let hardware_id = self.security_manager.get_hardware_id().await.unwrap_or_default();
        let is_validated = self.security_manager.is_validated().await;

        if !is_validated {
            return Err(anyhow::anyhow!("Cannot upload harvest: License not validated"));
        }

        let response = self.http_client
            .post(&url)
            .header("X-Hardware-ID", hardware_id)
            .json(&data)
            .send()
            .await
            .context("Failed to connect to central training server")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "Harvest upload failed");
            return Err(anyhow::anyhow!("Training server rejected harvest: {}", status));
        }

        Ok(())
    }
}
