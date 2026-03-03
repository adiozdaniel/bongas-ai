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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedRow {
    pub title: String,
    pub row_type: String, // e.g., "horizontal_list", "hero_carousel", "feature_grid"
    pub row_style: Option<String>, // e.g., "promotional", "compact", "tall_cards"
    pub scenario: String,
    pub items: Vec<RecommendationItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomeFeedResponse {
    pub rows: Vec<FeedRow>,
    pub experiment_id: Option<String>,
}
