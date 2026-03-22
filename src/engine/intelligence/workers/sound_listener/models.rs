//! Models for the Sound Listener and Linguistic workers.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Stage 1: Raw record of an audio transcription (The Ear).
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct AudioTranscript {
    pub item_id: i32,
    pub transcript: String,
    pub detected_language: String, // Whisper's best guess
    pub extracted_at: DateTime<Utc>,
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
    pub processed_at: DateTime<Utc>,
}
