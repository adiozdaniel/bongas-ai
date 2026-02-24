//! API data models.

pub mod recommendation;
pub mod response;
pub mod scenario;
pub mod system;

pub use recommendation::{RecommendationItem, FeedRow, HomeFeedResponse};
pub use response::{StandardResponse, ContextParams};
pub use scenario::{CreateScenarioRequest, UpdateScenarioRequest};
pub use system::{
    HealthResponse, CacheStatsResponse, KafkaMetricsResponse, KafkaHealthResponse,
    ModelReloadResponse, ModelStatsResponse, SecurityStatusResponse,
};
