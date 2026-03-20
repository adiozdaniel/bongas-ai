//! Models for the Sound Listener worker.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Forensic record of an audio transcription.
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct AudioTranscript {
    pub item_id: i32,
    pub transcript: String,
    pub extracted_at: DateTime<Utc>,
}
