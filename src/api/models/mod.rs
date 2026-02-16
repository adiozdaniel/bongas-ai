//! API models organized by domain.

pub mod response;
pub mod recommendation;
pub mod system;
pub mod scenario;

// Re-export all types for convenience
pub use response::{StandardResponse, ErrorBody, ResponseMeta, PaginationMeta, PaginationParams};
pub use recommendation::RecommendationItem;
pub use scenario::{CreateScenarioRequest, UpdateScenarioRequest};
pub use system::{
    HealthResponse, CacheStatsResponse, KafkaMetricsResponse, KafkaHealthResponse,
    ModelReloadResponse, ModelStatsResponse, SecurityStatusResponse,
};
