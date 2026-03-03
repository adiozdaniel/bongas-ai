//! Domain types for UI orchestration and page management.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents a UI page slug (e.g., "home", "movies", "tv_shows").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PageSlug(pub String);

impl std::fmt::Display for PageSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single row definition within a page composition.
/// dictating both content (scenario) and presentation (UI metadata).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCompositionItem {
    /// The engine scenario to execute for this row.
    pub slug: String,
    /// Fallback scenario if the primary fails (e.g., 'trending_now').
    pub fallback_slug: Option<String>,
    /// The UI component type (e.g., "hero_carousel", "horizontal_list").
    pub row_type: String,
    /// Visual styling hints (e.g., "promotional", "compact").
    pub row_style: Option<String>,
}

/// The layout definition for a specific page.
/// Defines which scenarios should be displayed, their UI hints, and targeting rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayout {
    pub page_slug: PageSlug,
    /// Targeted device type (e.g., "mobile", "tv", "default").
    pub device_type: Option<String>,
    /// Targeted maturity rating (e.g., "G", "18+").
    pub maturity_rating: Option<String>,
    /// Priority for resolver selection (higher is better).
    pub priority: i32,
    /// The structured list of scenarios and UI metadata.
    pub composition: Vec<PageCompositionItem>,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

/// Request to create or update a page layout.
#[derive(Debug, Deserialize)]
pub struct SavePageLayoutRequest {
    pub page_slug: String,
    pub device_type: Option<String>,
    pub maturity_rating: Option<String>,
    pub priority: Option<i32>,
    pub composition: Vec<PageCompositionItem>,
    pub is_active: Option<bool>,
}
