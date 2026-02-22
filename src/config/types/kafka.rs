//! Kafka configuration for the Composite Configuration Pattern.
//!
//! Provides configuration for Kafka connection settings and topic management.

use serde::{Deserialize, Serialize};

/// Kafka configuration.
///
/// Configuration for Kafka connection settings including brokers,
/// group ID, and topic configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub group_id: String,
    pub profile_topic: String,
    pub reaction_topic: String,
    pub notification_topic: String,
    pub playback_topic: String,
    pub connection_timeout: u64,
    pub request_timeout: u64,
    pub max_retries: u32,
    pub retry_backoff: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            group_id: "bongas-ai-consumers".to_string(),
            profile_topic: "user.profiles".to_string(),
            reaction_topic: "user.reactions".to_string(),
            notification_topic: "notifications".to_string(),
            playback_topic: "playback.sessions".to_string(),
            connection_timeout: 10,
            request_timeout: 30,
            max_retries: 3,
            retry_backoff: 1000,
        }
    }
}
