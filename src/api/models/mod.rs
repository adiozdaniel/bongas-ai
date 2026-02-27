//! API Models - Bongas-AI
//! This module aggregates all public API models used for requests and responses.
//! Organized by domain to maintain high cohesion and clear separation of concerns.

pub mod recommendation;
pub mod response;
pub mod scenario;
pub mod system;

// Re-export common types for convenience
pub use response::{StandardResponse, PaginationParams, ErrorBody, ResponseMeta, PaginationMeta};
pub use recommendation::RecommendationItem;
pub use scenario::{CreateScenarioRequest, UpdateScenarioRequest};
pub use system::{
    HealthResponse, CacheStatsResponse, KafkaMetricsResponse, 
    KafkaHealthResponse, ModelReloadResponse, ModelStatsResponse, 
    SecurityStatusResponse
};

use serde::Deserialize;

/// Shared context parameters used across various recommendation and engine endpoints.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct ContextParams {
    /// Unique identifier for the user profile.
    pub profile_id: Option<String>,
    /// Content maturity rating filter.
    pub maturity_rating: Option<String>,
    /// Device type for platform-specific optimizations.
    pub device_type: Option<String>,
}
