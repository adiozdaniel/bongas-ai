//! The Functional Core of the Forensics Pillar.
//! 
//! Manages the Forensic Ledger, mismatch reconciliation, and Sovereign Skepticism.

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, debug};
use clickhouse::Client as ClickHouseClient;

use crate::db::ResilientPool;
use crate::engine::intelligence::forensics::models::{
    SovereignSightLedger, AudioTranscript, ForensicMismatch
};

pub struct ForensicPillar {
    clickhouse: ClickHouseClient,
    db_pool: Arc<ResilientPool>,
}

impl ForensicPillar {
    pub fn new(clickhouse: ClickHouseClient, db_pool: Arc<ResilientPool>) -> Self {
        Self {
            clickhouse,
            db_pool,
        }
    }

    /// Record a visual DNA audit and predicted maturity rating.
    pub async fn record_visual_audit(&self, ledgers: Vec<SovereignSightLedger>) -> Result<()> {
        if ledgers.is_empty() { return Ok(()); }
        
        let mut insert = self.clickhouse.insert::<SovereignSightLedger>("sovereign_sight_ledger").await?;
        for row in ledgers {
            insert.write(&row).await?;
        }
        insert.end().await?;
        Ok(())
    }

    /// Record a raw audio transcription from The Ear.
    pub async fn record_audio_transcript(&self, transcript: AudioTranscript) -> Result<()> {
        let mut insert = self.clickhouse.insert::<AudioTranscript>("audio_transcripts").await?;
        insert.write(&transcript).await?;
        insert.end().await?;
        Ok(())
    }

    /// Compare an AI-predicted forensic rating against the manual database tag.
    /// If there is a mismatch (e.g., AI says 17+, DB says GE), log it to the audit ledger.
    pub async fn reconcile_maturity(&self, item_id: i32, forensic_rating: &str) -> Result<()> {
        // Fetch manual tag from Postgres
        let manual_rating: Option<String> = self.db_pool.execute(move |pool| async move {
            sqlx::query_scalar("SELECT maturity_rating FROM bongas.item_features WHERE item_id = $1")
                .bind(item_id)
                .fetch_optional(&pool)
                .await
        }).await?;

        if let Some(manual) = manual_rating {
            if forensic_rating != manual && forensic_rating == "17+" {
                info!(item_id, forensic_rating, manual, "Forensic mismatch detected");
                
                let mismatch = ForensicMismatch {
                    item_id,
                    forensic_rating: forensic_rating.to_string(),
                    manual_rating: manual,
                    detected_at: chrono::Utc::now().timestamp() as u64,
                };
                
                let mut insert = self.clickhouse.insert::<ForensicMismatch>("forensic_mismatches").await?;
                insert.write(&mismatch).await?;
                insert.end().await?;
            }
        }
        Ok(())
    }

    /// Sovereign Skepticism: Check if the AI strongly disagrees with a proposed manual tag.
    /// Used by the HiveMind to issue a "uko sure wewe?" challenge.
    pub async fn is_skeptical_of_override(&self, item_id: i32, proposed_rating: &str) -> Result<bool> {
        if proposed_rating == "17+" || proposed_rating == "18+" {
            return Ok(false); // If humans are making it stricter, we agree.
        }

        // Check the mismatch ledger
        let query = format!(
            "SELECT count() FROM forensic_mismatches WHERE item_id = {} AND forensic_rating IN ('17+', '18+')",
            item_id
        );
        
        match self.clickhouse.query(&query).fetch_one::<u64>().await {
            Ok(count) => Ok(count > 0),
            Err(e) => {
                debug!(error = %e, "Failed to check forensic skepticism state");
                Ok(false) // Fail-open
            }
        }
    }

    /// Fetch catalog IDs from Postgres (for differential census).
    pub async fn fetch_catalog_ids(&self) -> Result<Vec<i32>> {
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

    /// Fetch already audited ledger IDs from ClickHouse (for differential census).
    pub async fn fetch_audited_vision_ids(&self) -> Result<std::collections::HashSet<i32>> {
        #[derive(serde::Deserialize, clickhouse::Row)]
        struct ExternalIdRow { external_id: i32 }

        let rows: Vec<ExternalIdRow> = self.clickhouse
            .query("SELECT external_id FROM sovereign_sight_ledger")
            .fetch_all()
            .await?;
        Ok(rows.into_iter().map(|r| r.external_id).collect())
    }
}