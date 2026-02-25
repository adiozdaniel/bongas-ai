use serde_json::Value as JsonValue;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// Full item features row — superset of all columns pipeline stages may need.
#[derive(Debug, Clone, FromRow)]
pub struct ItemFeatureRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub genres: Option<JsonValue>,
    pub tags: Option<JsonValue>,
    pub creators: Option<JsonValue>,
    pub directors: Option<JsonValue>,
    pub studios: Option<JsonValue>,
    pub actors: Option<JsonValue>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub audio_languages: Option<JsonValue>,
    pub subtitle_languages: Option<JsonValue>,
    pub age_rating: Option<String>,
    pub duration_seconds: Option<i32>,
    pub release_year: Option<i32>,
    pub release_date: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub added_date: Option<DateTime<Utc>>,
    pub available_from: Option<DateTime<Utc>>,
    pub available_until: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub max_resolution: Option<String>,
    pub has_hdr: Option<bool>,
    pub has_dolby_vision: Option<bool>,
    pub has_dolby_atmos: Option<bool>,
    pub is_explicit: Option<bool>,
    pub has_violence: Option<bool>,
    pub has_strong_language: Option<bool>,
    pub has_drug_content: Option<bool>,
    pub available_countries: Option<JsonValue>,
    pub blocked_countries: Option<JsonValue>,
    pub seasonal_tags: Option<JsonValue>,
    pub holiday_tags: Option<JsonValue>,
    pub themes: Option<JsonValue>,
    pub is_award_winner: Option<bool>,
    pub required_tier: Option<String>,
    pub is_free: Option<bool>,
    pub view_count: i32,
    pub like_count: i32,
    pub comment_count: Option<i64>,
    pub share_count: Option<i64>,
    pub save_count: Option<i64>,
    pub completion_rate: f32,
    pub trending_score: f32,
    pub popularity_score: Option<f32>,
    pub user_rating: Option<f32>,
    pub user_rating_count: Option<i32>,
    pub critic_rating: Option<f32>,
    pub critic_rating_count: Option<i32>,
    pub embedding: Option<Vec<f32>>,
    pub tfidf_vector: Option<JsonValue>,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserFeatureRow {
    pub user_id: i32,
    pub genre_affinity: Option<JsonValue>,
    pub disliked_genres: Option<JsonValue>,
    pub total_watch_time_minutes: i32,
    pub total_videos_watched: i32,
    pub avg_completion_rate: f32,
    pub favorite_genres: Option<JsonValue>,
    pub favorite_creators: Option<JsonValue>,
    pub preferred_content_type: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WatchedItemRow {
    pub item_id: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserContentPreferencesRow {
    pub allow_explicit: Option<bool>,
    pub allow_violence: Option<bool>,
    pub allow_language: Option<bool>,
    pub allow_drugs: Option<bool>,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserProfileRow {
    pub segment: Option<String>,
    pub subscription_tier: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PromotionRow {
    pub item_id: i32,
    pub promotion_priority: Option<i32>,
    pub target_segments: Option<JsonValue>,
    pub promotion_label: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserItemScoreRow {
    pub item_id: i32,
    pub score: f32,
    pub model_type: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct RecentWatchRow {
    pub item_id: i32,
    pub last_watched: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PopularItemRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub is_explicit: Option<bool>,
    pub view_count: i32,
    pub trending_score: f32,
    pub completion_rate: f32,
    pub age_rating: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub genres: Option<JsonValue>,
}

#[derive(Debug, Clone, FromRow)]
pub struct NewReleaseRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub trending_score: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct GenreItemRow {
    pub item_id: i32,
    pub title: Option<String>,
    pub popularity_score: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SeasonalItemRowExtended {
    pub item_id: i32,
    pub title: Option<String>,
    pub seasonal_tags: Option<JsonValue>,
    pub holiday_tags: Option<JsonValue>,
    pub themes: Option<JsonValue>,
    pub popularity_score: Option<f32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ItemSimilarityRow {
    pub similar_item_id: i32,
    pub similarity_score: f32,
}

#[derive(Debug, Clone, FromRow)]
pub struct WatchlistItemRow {
    pub item_id: i32,
    pub added_at: DateTime<Utc>,
}
