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

/// The layout definition for a specific page.
/// Defines which scenarios should be displayed and in what order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayout {
    pub page_slug: PageSlug,
    pub scenario_slugs: Vec<String>,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

/// Request to create or update a page layout.
#[derive(Debug, Deserialize)]
pub struct SavePageLayoutRequest {
    pub page_slug: String,
    pub scenario_slugs: Vec<String>,
    pub is_active: Option<bool>,
}
