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
