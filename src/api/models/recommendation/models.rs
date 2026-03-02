//! Recommendation domain models.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RecommendationItem {
    pub item_id: i32,
    pub title: String,
    pub thumbnail_url: String,
    pub score: f32,
    pub rank: i32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct FeedRow {
    pub title: String,
    pub row_type: String, // e.g., "horizontal_list", "hero_carousel", "feature_grid"
    pub row_style: Option<String>, // e.g., "promotional", "compact", "tall_cards"
    pub scenario: String,
    pub items: Vec<RecommendationItem>,
}

#[derive(Debug, Serialize)]
pub struct HomeFeedResponse {
    pub rows: Vec<FeedRow>,
    pub experiment_id: Option<String>,
}
