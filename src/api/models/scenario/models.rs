use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScenarioRequest {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub target_kpi: Option<String>,
    pub maturity_rating: Option<String>,
    pub pipeline: JsonValue,
    pub initial_display_limit: Option<i32>,
    pub scope: Option<JsonValue>,
    pub cache_ttl_seconds: Option<i32>,
    pub use_l2_cache: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScenarioRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub target_kpi: Option<Option<String>>,
    pub maturity_rating: Option<String>,
    pub pipeline: Option<JsonValue>,
    pub initial_display_limit: Option<i32>,
    pub scope: Option<JsonValue>,
    pub cache_ttl_seconds: Option<i32>,
    pub use_l2_cache: Option<bool>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}
