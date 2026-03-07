//! Recommendation domain models.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationItem {
    pub item_id: i32,
    pub title: String,
    pub thumbnail_url: String,
    pub score: f32,
    pub rank: i32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestEvent {
    pub event: String, // 'click', 'playback', 'reaction', 'impression'
    pub user_id: Option<i32>,
    pub item_id: i32,
    pub scenario: Option<String>,
    pub watch_percentage: Option<f32>,
    pub reaction_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymphonyNavigation {
    pub slug: String,
    pub title: String,
    pub nav_type: String,
    pub nav_mesh: Vec<SymphonyNavigation>,
    pub landing_slug: String,
    pub total_rows: usize,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRow {
    pub title: String,
    pub row_type: String,
    pub row_style: Option<String>,
    pub scenario: String,
    pub scenario_slug: String,
    pub items: Vec<RecommendationItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryManifest {
    pub total_rows: usize,
    pub batch_size: i32,
    pub prewarming_active: bool,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContinuationEvent {
    pub next_url: String,
    pub next_offset: usize,
    pub next_batch: i32,
}
