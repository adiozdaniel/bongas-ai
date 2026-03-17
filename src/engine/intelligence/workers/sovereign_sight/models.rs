use serde::{Serialize, Deserialize};

/// Represents the visual intelligence record from the Sovereign Sight native worker.
///
/// This ledger acts as our internal Source of Truth for visual DNA, maturity forensics,
/// and semantic vibes, linked to the client's catalog via `external_id`.
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct SovereignSightLedger {
    pub external_id: i32,
    pub content_type: String,
    
    // Step 3: Visual DNA Extraction
    pub visual_dna: Vec<f32>,
    pub motion_entropy: f32,
    pub appearance_dna: String,
    
    // Step 4: Forensic Maturity Auditor
    pub maturity_rating: String,
    pub maturity_reason: String,
    
    // Step 5: Semantic Digest Engine (Layman Report)
    pub semantic_digest: String,
    
    // Step 6: Hook & Teaser Factory
    pub hook_path: String,
}
