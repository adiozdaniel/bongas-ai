//! Centralized models for the Forensics Pillar.

use serde::{Serialize, Deserialize};

/// Represents the visual intelligence record from the Forensic Auditor.
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct SovereignSightLedger {
    pub external_id: i32,
    pub content_type: String,
    
    // Visual DNA Extraction
    pub visual_dna: Vec<f32>,
    pub motion_entropy: f32,
    pub appearance_dna: String,
    
    // Forensic Maturity Auditor
    pub maturity_rating: String,
    pub maturity_reason: String,
    
    // Semantic Digest Engine (Layman Report)
    pub semantic_digest: String,
    
    // Hook & Teaser Factory
    pub hook_path: String,
}

/// Stage 1: Raw record of an audio transcription (The Ear).
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct AudioTranscript {
    pub item_id: i32,
    pub transcript: String,
    pub audio_dna: Option<Vec<f32>>,
    pub detected_language: String, // Whisper's best guess
    pub extracted_at: chrono::DateTime<chrono::Utc>,
    pub processed: bool,
}

/// Stage 2: Mapped and translated linguistic record (The Linguist).
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct ProcessedAudioIntelligence {
    pub item_id: i32,
    pub native_text: String,
    pub english_translation: String,
    pub language_id: String,
    pub dialect_mappings: String, // JSON-serialized map of dialect -> standardized words
    pub processed_at: chrono::DateTime<chrono::Utc>,
}

/// A forensic discrepancy between AI prediction and human metadata.
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct ForensicMismatch {
    pub item_id: i32,
    pub forensic_rating: String,
    pub manual_rating: String,
    pub detected_at: u64, // Unix timestamp for ClickHouse compatibility
}
