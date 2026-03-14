//! Notification Models and DTOs.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailPayload {
    pub profile_id: String,
    pub email: String,
    pub subject: String,
    pub body_html: String,
    pub template_slug: String,
    pub metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PendingEmailRow {
    pub id: i32,
    pub profile_id: String,
    pub email: String,
    pub subject: String,
    pub body_html: String,
    pub template_slug: String,
    pub metadata: JsonValue,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxNotification {
    pub profile_id: String,
    pub title: String,
    pub message: String,
    pub action_url: Option<String>,
    pub category: String,
    pub priority: i32,
    pub metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InboxNotificationRow {
    pub id: i32,
    pub profile_id: String,
    pub title: String,
    pub message: String,
    pub action_url: Option<String>,
    pub category: String,
    pub priority: i32,
    pub metadata: JsonValue,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationIntent {
    Email(EmailPayload),
    Inbox(InboxNotification),
}

impl NotificationIntent {
    pub fn profile_id(&self) -> &str {
        match self {
            Self::Email(p) => &p.profile_id,
            Self::Inbox(p) => &p.profile_id,
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            Self::Email(_) => "email",
            Self::Inbox(_) => "inbox",
        }
    }
}

/// Record for ClickHouse ledger.
#[derive(Debug, Clone, Serialize, Deserialize, clickhouse::Row)]
pub struct NotificationLedgerEntry {
    pub profile_id: String,
    pub notification_type: String,
    pub template_slug: Option<String>,
    pub category: Option<String>,
    pub status: String,
    pub created_at: i64,
}
