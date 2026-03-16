use serde::{Deserialize, Serialize};

/// Configuration for Meilisearch integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Host URL for the Meilisearch instance.
    pub host: String,
    /// API Key for authenticated access.
    pub api_key: String,
    /// Primary index name for content.
    pub index_name: String,
    /// Timeout for search requests in milliseconds.
    pub timeout_ms: u64,
    /// Maximum number of hits per request.
    pub max_hits: usize,
    /// Whether to enable typo tolerance.
    pub typo_tolerance: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            host: "http://localhost:7700".to_string(),
            api_key: "".to_string(),
            index_name: "items".to_string(),
            timeout_ms: 500,
            max_hits: 100,
            typo_tolerance: true,
        }
    }
}
