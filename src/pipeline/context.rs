use std::sync::Arc;
use sqlx::PgPool;
use crate::analytics::ClickHouseClient;
use crate::cache::redis::RedisClient;

/// Execution context passed to all stages
pub struct ExecutionContext {
    pub user_id: Option<i32>,
    pub device_type: Option<String>,
    pub location: Option<String>,

    // Dependencies
    pub db_pool: Arc<PgPool>,
    pub clickhouse: Arc<ClickHouseClient>,
    pub redis: Arc<RedisClient>,

    // Request metadata
    pub request_id: String,
}
