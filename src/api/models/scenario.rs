use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScenarioRequest {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub pipeline: JsonValue,
    pub cache_ttl_seconds: Option<i32>,
    pub use_l2_cache: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScenarioRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub pipeline: Option<JsonValue>,
    pub cache_ttl_seconds: Option<i32>,
    pub use_l2_cache: Option<bool>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}
