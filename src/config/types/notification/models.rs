//! Notification configuration for the Composite Configuration Pattern.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub adaptor: NotificationAdaptorKind,
    pub resend: ResendConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAdaptorKind {
    Kafka,
    Polling,
    Resend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResendConfig {
    pub api_key: String,
    pub from_email: String,
    pub from_name: String,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            adaptor: NotificationAdaptorKind::Polling,
            resend: ResendConfig::default(),
        }
    }
}

impl Default for ResendConfig {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            from_email: "noreply@bongas-ai.com".to_string(),
            from_name: "Bongas-AI".to_string(),
        }
    }
}
