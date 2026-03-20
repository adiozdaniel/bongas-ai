use serde::{Deserialize, Serialize};

/// Configuration for Embedded Search (Tantivy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Whether embedded search is enabled.
    pub enabled: bool,
    /// Path to the local search index directory.
    pub index_path: String,
    /// Memory budget for the index writer in MB.
    pub writer_memory_mb: usize,
    /// Primary index name for content.
    pub index_name: String,
    /// Timeout for search requests in milliseconds.
    pub timeout_ms: u64,
    /// Maximum number of hits per request.
    pub max_hits: usize,
    /// Whether to enable typo tolerance (simulated in Tantivy via fuzzy queries).
    pub typo_tolerance: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            index_path: "data/search_index".to_string(),
            writer_memory_mb: 50,
            index_name: "items".to_string(),
            timeout_ms: 500,
            max_hits: 100,
            typo_tolerance: true,
        }
    }
}
