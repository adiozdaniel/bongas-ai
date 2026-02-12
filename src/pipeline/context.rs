use std::sync::Arc;
use sqlx::PgPool;
use crate::analytics::ClickHouseClient;
use crate::cache::redis::RedisClient;
use crate::ml::model_loader::ModelLoader;

/// Execution context passed to all pipeline stages
pub struct ExecutionContext {
    pub user_id: Option<i32>,
    pub device_type: Option<String>,
    pub location: Option<String>,
    pub _profile_context: Option<ProfileContext>,

    // Dependencies
    pub db_pool: Arc<PgPool>,
    pub clickhouse: Arc<ClickHouseClient>,
    pub redis: Arc<RedisClient>,
    pub model_loader: Arc<ModelLoader>,  // ◄── NEW: ONNX model access

    // Request metadata
    pub request_id: String,
}

/// Additional profile context for personalization
#[derive(Debug, Clone, Default)]
pub struct ProfileContext {
    pub _age_group: Option<String>,
    pub _subscription_tier: Option<String>,
    pub _preferred_languages: Vec<String>,
    pub _content_preferences: serde_json::Value,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(
        user_id: Option<i32>,
        db_pool: Arc<PgPool>,
        clickhouse: Arc<ClickHouseClient>,
        redis: Arc<RedisClient>,
        model_loader: Arc<ModelLoader>,
        request_id: String,
    ) -> Self {
        Self {
            user_id,
            device_type: None,
            location: None,
            _profile_context: None,
            db_pool,
            clickhouse,
            redis,
            model_loader,
            request_id,
        }
    }

    /// Builder pattern for optional fields
    pub fn with_device_type(mut self, device_type: String) -> Self {
        self.device_type = Some(device_type);
        self
    }

    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }

}
