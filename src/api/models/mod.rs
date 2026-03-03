//! API Models - Bongas-AI
//! This module aggregates all public API models used for requests and responses.
//! Organized by domain to maintain high cohesion and clear separation of concerns.

pub mod recommendation;
pub mod response;
pub mod scenario;
pub mod system;

// Re-export common types for unified access
pub use response::{StandardResponse, PaginationParams, ErrorBody, ResponseMeta, PaginationMeta};
pub use recommendation::{RecommendationItem, FeedRow, IngestEvent, SymphonyNavigation};
pub use scenario::{CreateScenarioRequest, UpdateScenarioRequest};
pub use system::{
    HealthResponse, CacheStatsResponse, KafkaMetricsResponse, 
    KafkaHealthResponse, ModelReloadResponse, ModelStatsResponse, 
    SecurityStatusResponse, ContextParams
};
