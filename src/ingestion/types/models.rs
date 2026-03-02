//! Core types for the activity ingestion backbone.
//!
//! Transport-agnostic: these types represent user activities regardless of
//! whether they arrived via Kafka, a direct API call, or a ClickHouse backfill.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ─── User Activities (transport-agnostic) ───────────────────────────────────

/// A user activity received from any source.
///
/// This is the universal currency of the ingestion layer. Every source
/// converts its native format into one of these variants before handing
/// it to the processor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserActivity {
    /// User watched content (playback session).
    Playback {
        user_id: i32,
        item_id: i32,
        session_id: String,
        visitor_id: Option<String>,
        device_hash: Option<String>,
        watch_duration_seconds: i32,
        total_duration_seconds: i32,
        watch_percentage: f32,
        completed: bool,
        scenario_slug: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User liked/disliked content.
    Reaction {
        user_id: i32,
        item_id: i32,
        visitor_id: Option<String>,
        device_hash: Option<String>,
        /// "like", "dislike", or other reaction types.
        reaction_type: String,
        scenario_slug: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User profile was updated.
    ProfileUpdate {
        user_id: i32,
        /// "preferences", "settings", "demographics", "subscription"
        update_type: String,
        data: serde_json::Value,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// A notification-worthy event occurred.
    Notification {
        user_id: i32,
        notification_type: String,
        title: String,
        body: String,
        data: serde_json::Value,
        priority: String,
        channels: Vec<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User clicked on a content item.
    Click {
        user_id: i32,
        item_id: i32,
        visitor_id: Option<String>,
        device_hash: Option<String>,
        scenario_slug: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Content was shown to a user (impression tracking).
    Impression {
        user_id: i32,
        item_id: i32,
        visitor_id: Option<String>,
        device_hash: Option<String>,
        scenario_slug: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl UserActivity {
    /// The user this activity belongs to (if applicable).
    pub fn user_id(&self) -> i32 {
        match self {
            Self::Playback { user_id, .. }
            | Self::Reaction { user_id, .. }
            | Self::ProfileUpdate { user_id, .. }
            | Self::Notification { user_id, .. }
            | Self::Click { user_id, .. }
            | Self::Impression { user_id, .. } => *user_id,
        }
    }

    /// The visitor ID associated with this activity.
    pub fn visitor_id(&self) -> Option<&str> {
        match self {
            Self::Playback { visitor_id, .. }
            | Self::Reaction { visitor_id, .. }
            | Self::Click { visitor_id, .. }
            | Self::Impression { visitor_id, .. } => visitor_id.as_deref(),
            _ => None,
        }
    }

    /// The device hash associated with this activity.
    pub fn device_hash(&self) -> Option<&str> {
        match self {
            Self::Playback { device_hash, .. }
            | Self::Reaction { device_hash, .. }
            | Self::Click { device_hash, .. }
            | Self::Impression { device_hash, .. } => device_hash.as_deref(),
            _ => None,
        }
    }

    /// The scenario slug associated with this activity.
    pub fn scenario_slug(&self) -> Option<&str> {
        match self {
            Self::Playback { scenario_slug, .. }
            | Self::Reaction { scenario_slug, .. }
            | Self::Click { scenario_slug, .. }
            | Self::Impression { scenario_slug, .. } => scenario_slug.as_deref(),
            _ => None,
        }
    }

    /// Short label for logging/metrics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Playback { .. } => "playback",
            Self::Reaction { .. } => "reaction",
            Self::ProfileUpdate { .. } => "profile_update",
            Self::Notification { .. } => "notification",
            Self::Click { .. } => "click",
            Self::Impression { .. } => "impression",
        }
    }
}

// ─── Activity Source Trait ───────────────────────────────────────────────────

/// Health status for a single ingestion source.
#[derive(Debug, Clone, Serialize)]
pub struct SourceHealth {
    pub source_name: String,
    pub healthy: bool,
    pub messages_ingested: u64,
    pub errors: u64,
    pub circuit_state: String,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
}

/// Trait for activity sources (Kafka, API, ClickHouse polling, etc.).
///
/// Each source converts its native event format into `UserActivity` and
/// pushes it into the shared channel. The `ActivityProcessor` on the other
/// end handles normalization, DB writes, and staleness invalidation.
#[async_trait::async_trait]
pub trait ActivitySource: Send + Sync {
    /// Human-readable name for logging and metrics (e.g. "kafka", "api", "clickhouse").
    fn name(&self) -> &str;

    /// Start producing activities into the provided channel.
    ///
    /// This method should run until cancelled (via `tokio::select!` or task abort).
    /// It must not panic — errors should be logged and retried according to the
    /// source's circuit breaker state.
    async fn start(&self, sender: mpsc::Sender<UserActivity>) -> Result<()>;

    /// Report current health status.
    async fn health(&self) -> SourceHealth;
}
