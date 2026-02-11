//! Common test utilities and fixtures for BONGAS-AI tests
//!
//! This module provides shared test infrastructure including:
//! - Test database setup and teardown
//! - Test data generators
//! - Mock implementations
//! - Test configuration
//! - Helper functions for common test patterns

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::{PgPool, postgres::PgPoolOptions};
use redis::Client as RedisClient;
use clickhouse::Client as ClickHouseClient;

use bongas_ai::cache::redis::RedisClient as BongasRedisClient;
use bongas_ai::cache::postgres_cache::PostgresCache;
use bongas_ai::db::repositories::cache_repository::CacheRepository;
use bongas_ai::engine::staging_manager::StagingManager;
use bongas_ai::pipeline::ScoredItem;
use bongas_ai::config::settings::Settings;

/// Test configuration
pub struct TestConfig {
    pub db_url: String,
    pub redis_url: String,
    pub clickhouse_url: String,
    pub kafka_brokers: String,
}

impl TestConfig {
    pub fn new() -> Self {
        Self {
            db_url: std::env::var("TEST_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5433/bongas_ai_test".to_string()),
            redis_url: std::env::var("TEST_REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6380".to_string()),
            clickhouse_url: std::env::var("TEST_CLICKHOUSE_URL")
                .unwrap_or_else(|_| "http://localhost:8124".to_string()),
            kafka_brokers: std::env::var("TEST_KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9093".to_string()),
        }
    }

    pub fn with_prefix(prefix: &str) -> Self {
        let base = Self::new();
        Self {
            db_url: format!("{}?application_name={}", base.db_url, prefix),
            ..base
        }
    }
}

/// Test database setup
pub async fn setup_test_db(config: &TestConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&config.db_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}

/// Test Redis client setup
pub async fn setup_test_redis(config: &TestConfig) -> Result<Arc<BongasRedisClient>> {
    let client = RedisClient::open(&config.redis_url)?;
    let redis_client = BongasRedisClient::new(client).await?;
    Ok(Arc::new(redis_client))
}

/// Test ClickHouse client setup
pub async fn setup_test_clickhouse(config: &TestConfig) -> Result<ClickHouseClient> {
    let client = ClickHouseClient::default()
        .with_url(&config.clickhouse_url);
    
    // Test connection
    client.query("SELECT 1").fetch_one::<i32>().await?;
    
    Ok(client)
}

/// Test staging manager setup
pub async fn setup_test_staging_manager(
    db_pool: PgPool,
    redis_client: Arc<BongasRedisClient>,
) -> Result<StagingManager> {
    let cache_repo = Arc::new(CacheRepository::new(db_pool.clone()));
    let postgres_cache = Arc::new(PostgresCache::new(db_pool));
    
    let staging_manager = StagingManager::new(
        cache_repo,
        postgres_cache,
        redis_client,
        Duration::from_secs(300), // 5 minute TTL
        Duration::from_secs(3600), // 1 hour L2 TTL
    );

    Ok(staging_manager)
}

/// Test data generators
pub mod fixtures {
    use super::*;

    /// Create test scored items
    pub fn create_test_items(count: usize) -> Vec<ScoredItem> {
        (1..=count)
            .map(|i| ScoredItem {
                item_id: i as i64,
                score: 1.0 - (i as f32 / 100.0),
                metadata: JsonValue::Object(serde_json::Map::new()),
            })
            .collect()
    }

    /// Create test items with metadata
    pub fn create_test_items_with_metadata(count: usize) -> Vec<ScoredItem> {
        (1..=count)
            .map(|i| ScoredItem {
                item_id: i as i64,
                score: 1.0 - (i as f32 / 100.0),
                metadata: serde_json::json!({
                    "title": format!("Test Item {}", i),
                    "genre": if i % 3 == 0 { "Action" } else if i % 3 == 1 { "Drama" } else { "Comedy" },
                    "year": 2020 + (i % 10) as i32,
                    "duration": 90 + (i * 10) as i32,
                }),
            })
            .collect()
    }

    /// Create test scenario configuration
    pub fn create_test_scenario_config() -> JsonValue {
        serde_json::json!({
            "stages": [
                {
                    "type": "clickhouse_trending",
                    "params": {
                        "limit": 20,
                        "time_window": "1h",
                        "min_interactions": 5
                    }
                },
                {
                    "type": "ml_inference",
                    "params": {
                        "model_name": "two_tower_v1",
                        "model_format": "onnx",
                        "top_k": 10
                    }
                },
                {
                    "type": "filter_watched",
                    "params": {
                        "max_watched": 50
                    }
                },
                {
                    "type": "diversify_by_genre",
                    "params": {
                        "max_same_genre": 3
                    }
                }
            ]
        })
    }

    /// Create test user context
    pub fn create_test_context() -> JsonValue {
        serde_json::json!({
            "user_id": 123,
            "device_type": "mobile",
            "location": "kenya",
            "time_of_day": "evening",
            "session_id": "test_session_123"
        })
    }

    /// Create test user features
    pub fn create_test_user_features() -> Vec<f32> {
        vec![0.1; 64] // 64-dimensional user embedding
    }

    /// Create test item features
    pub fn create_test_item_features() -> Vec<f32> {
        vec![0.2; 32] // 32-dimensional item embedding
    }
}

/// Test helpers for database operations
pub mod db_helpers {
    use super::*;

    /// Seed test data into database
    pub async fn seed_test_data(pool: &PgPool) -> Result<()> {
        // Seed users
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, created_at, updated_at)
            VALUES 
                (123, 'test_user', 'test@example.com', NOW(), NOW()),
                (456, 'test_user2', 'test2@example.com', NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .execute(pool)
        .await?;

        // Seed items
        sqlx::query!(
            r#"
            INSERT INTO items (id, title, genre, year, duration, created_at, updated_at)
            VALUES 
                (1, 'Test Movie 1', 'Action', 2023, 120, NOW(), NOW()),
                (2, 'Test Movie 2', 'Drama', 2022, 110, NOW(), NOW()),
                (3, 'Test Movie 3', 'Comedy', 2023, 100, NOW(), NOW()),
                (4, 'Test Movie 4', 'Action', 2021, 130, NOW(), NOW()),
                (5, 'Test Movie 5', 'Drama', 2023, 90, NOW(), NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .execute(pool)
        .await?;

        // Seed interactions
        sqlx::query!(
            r#"
            INSERT INTO interactions (user_id, item_id, interaction_type, score, created_at)
            VALUES 
                (123, 1, 'watch', 0.8, NOW()),
                (123, 2, 'like', 1.0, NOW()),
                (123, 3, 'dislike', 0.0, NOW()),
                (456, 1, 'watch', 0.9, NOW()),
                (456, 4, 'like', 1.0, NOW())
            ON CONFLICT DO NOTHING
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Clean up test data
    pub async fn cleanup_test_data(pool: &PgPool) -> Result<()> {
        sqlx::query!("DELETE FROM interactions WHERE user_id IN (123, 456)")
            .execute(pool)
            .await?;

        sqlx::query!("DELETE FROM items WHERE id BETWEEN 1 AND 5")
            .execute(pool)
            .await?;

        sqlx::query!("DELETE FROM users WHERE id IN (123, 456)")
            .execute(pool)
            .await?;

        Ok(())
    }
}

/// Test helpers for Redis operations
pub mod redis_helpers {
    use super::*;

    /// Clear all Redis keys for testing
    pub async fn clear_redis(redis_client: &BongasRedisClient) -> Result<()> {
        redis_client.flushdb().await?;
        Ok(())
    }

    /// Set test cache entry
    pub async fn set_test_cache(
        redis_client: &BongasRedisClient,
        key: &str,
        value: &JsonValue,
    ) -> Result<()> {
        redis_client.set_json(key, value, Duration::from_secs(300)).await?;
        Ok(())
    }
}

/// Test helpers for ClickHouse operations
pub mod clickhouse_helpers {
    use super::*;

    /// Create test ClickHouse tables
    pub async fn setup_test_tables(client: &ClickHouseClient) -> Result<()> {
        client.query(
            r#"
            CREATE TABLE IF NOT EXISTS test_interactions (
                user_id UInt64,
                item_id UInt64,
                interaction_type String,
                score Float32,
                timestamp DateTime
            ) ENGINE = MergeTree()
            ORDER BY (user_id, timestamp)
            "#,
        )
        .execute()
        .await?;

        Ok(())
    }

    /// Seed test ClickHouse data
    pub async fn seed_test_data(client: &ClickHouseClient) -> Result<()> {
        let mut insert = client.insert("test_interactions")?;
        
        for i in 1..=10 {
            insert.write(&(
                123u64, // user_id
                i as u64, // item_id
                "watch", // interaction_type
                0.8f32, // score
                chrono::Utc::now().naive_utc(), // timestamp
            )).await?;
        }

        insert.end().await?;
        Ok(())
    }

    /// Clean up test ClickHouse data
    pub async fn cleanup_test_data(client: &ClickHouseClient) -> Result<()> {
        client.query("TRUNCATE TABLE test_interactions")
            .execute()
            .await?;
        Ok(())
    }
}

/// Test transaction helper for database isolation
pub struct TestTransaction {
    pool: PgPool,
}

impl TestTransaction {
    pub async fn new(pool: PgPool) -> Result<Self> {
        Ok(Self { pool })
    }

    pub async fn with_transaction<F, Fut>(&self, test_fn: F) -> Result<()>
    where
        F: FnOnce(sqlx::Transaction<'_, sqlx::Postgres>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = test_fn(tx).await;
        tx.rollback().await?;
        result
    }
}

/// Test metrics helper
pub mod metrics {
    use super::*;

    /// Wait for cache metrics to update
    pub async fn wait_for_cache_metrics(staging_manager: &StagingManager, timeout: Duration) -> Result<()> {
        let start = SystemTime::now();
        
        loop {
            let stats = staging_manager.get_stats();
            if stats.l1_hits > 0 || stats.l2_hits > 0 || SystemTime::now().duration_since(start)? > timeout {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Assert cache hit rate is above threshold
    pub fn assert_cache_hit_rate(stats: &crate::cache::staging_manager::StagingStats, threshold: f64) {
        let hit_rate = stats.hit_rate;
        assert!(
            hit_rate >= threshold,
            "Cache hit rate {:.2}% is below threshold {:.2}%",
            hit_rate * 100.0,
            threshold * 100.0
        );
    }
}

/// Test configuration for different environments
pub mod test_env {
    use super::*;

    /// Get test configuration for CI environment
    pub fn ci_config() -> TestConfig {
        TestConfig::with_prefix("ci_test")
    }

    /// Get test configuration for local development
    pub fn local_config() -> TestConfig {
        TestConfig::with_prefix("local_test")
    }

    /// Get test configuration for Docker Compose environment
    pub fn docker_config() -> TestConfig {
        TestConfig {
            db_url: "postgresql://postgres:password@postgres:5432/bongas_ai_test".to_string(),
            redis_url: "redis://redis:6379".to_string(),
            clickhouse_url: "http://clickhouse:8123".to_string(),
            kafka_brokers: "kafka:9092".to_string(),
        }
    }
}

/// Mock implementations for testing
pub mod mocks {
    use super::*;
    use mockall::mock;

    mock! {
        pub ClickHouseClient {}

        #[async_trait::async_trait]
        impl clickhouse::Client for ClickHouseClient {
            type Query = clickhouse::query::Query<'static>;

            fn query(&self, query: &str) -> Self::Query {
                unimplemented!()
            }

            fn insert<T>(&self, table: &str) -> Result<clickhouse::insert::Insert<'_, T>, clickhouse::error::Error>
            where
                T: clickhouse::Row + Send,
            {
                unimplemented!()
            }
        }
    }

    mock! {
        pub RedisClient {}

        #[async_trait::async_trait]
        impl crate::cache::redis::RedisClient for RedisClient {
            async fn get_json<T>(&self, key: &str) -> Result<Option<T>, anyhow::Error>
            where
                T: serde::de::DeserializeOwned,
            {
                unimplemented!()
            }

            async fn set_json<T>(&self, key: &str, value: &T, ttl: Duration) -> Result<(), anyhow::Error>
            where
                T: serde::Serialize,
            {
                unimplemented!()
            }

            async fn del(&self, key: &str) -> Result<bool, anyhow::Error> {
                unimplemented!()
            }

            async fn flushdb(&self) -> Result<(), anyhow::Error> {
                unimplemented!()
            }

            async fn exists(&self, key: &str) -> Result<bool, anyhow::Error> {
                unimplemented!()
            }

            async fn keys(&self, pattern: &str) -> Result<Vec<String>, anyhow::Error> {
                unimplemented!()
            }

            async fn mget_json<T>(&self, keys: &[&str]) -> Result<Vec<Option<T>>, anyhow::Error>
            where
                T: serde::de::DeserializeOwned,
            {
                unimplemented!()
            }

            async fn mset_json<T>(&self, key_values: &[(&str, &T)], ttl: Duration) -> Result<(), anyhow::Error>
            where
                T: serde::Serialize,
            {
                unimplemented!()
            }

            async fn del_pattern(&self, pattern: &str) -> Result<u64, anyhow::Error> {
                unimplemented!()
            }
        }
    }
}

/// Test utilities for async testing
pub mod async_utils {
    use super::*;

    /// Timeout wrapper for async operations
    pub async fn with_timeout<F, T>(future: F, timeout: Duration) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| anyhow::anyhow!("Operation timed out"))?
    }

    /// Retry helper for flaky operations
    pub async fn retry_with_backoff<F, T>(
        mut operation: F,
        max_retries: u32,
        base_delay: Duration,
    ) -> Result<T>
    where
        F: FnMut() -> Box<dyn std::future::Future<Output = Result<T>> + Unpin>,
    {
        let mut delay = base_delay;
        
        for attempt in 1..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < max_retries => {
                    tokio::time::sleep(delay).await;
                    delay = delay * 2; // Exponential backoff
                }
                Err(e) => return Err(e),
            }
        }

        Err(anyhow::anyhow!("Operation failed after {} attempts", max_retries))
    }
}

/// Test data cleanup helper
pub struct TestDataGuard {
    cleanup_fn: Box<dyn FnOnce() -> Box<dyn std::future::Future<Output = Result<()>> + Send>>,
}

impl TestDataGuard {
    pub fn new<F>(cleanup_fn: F) -> Self
    where
        F: FnOnce() -> Box<dyn std::future::Future<Output = Result<()>> + Send> + Send + 'static,
    {
        Self {
            cleanup_fn: Box::new(cleanup_fn),
        }
    }
}

impl Drop for TestDataGuard {
    fn drop(&mut self) {
        // Note: In a real implementation, you'd want to handle async cleanup properly
        // This is a simplified version for the example
    }
}

/// Test runner with setup and teardown
pub struct TestRunner {
    config: TestConfig,
    db_pool: Option<PgPool>,
    redis_client: Option<Arc<BongasRedisClient>>,
    clickhouse_client: Option<ClickHouseClient>,
}

impl TestRunner {
    pub async fn new() -> Result<Self> {
        let config = TestConfig::new();
        Ok(Self {
            config,
            db_pool: None,
            redis_client: None,
            clickhouse_client: None,
        })
    }

    pub async fn setup_db(&mut self) -> Result<&PgPool> {
        if self.db_pool.is_none() {
            self.db_pool = Some(setup_test_db(&self.config).await?);
        }
        Ok(self.db_pool.as_ref().unwrap())
    }

    pub async fn setup_redis(&mut self) -> Result<&Arc<BongasRedisClient>> {
        if self.redis_client.is_none() {
            self.redis_client = Some(setup_test_redis(&self.config).await?);
        }
        Ok(self.redis_client.as_ref().unwrap())
    }

    pub async fn setup_clickhouse(&mut self) -> Result<&ClickHouseClient> {
        if self.clickhouse_client.is_none() {
            self.clickhouse_client = Some(setup_test_clickhouse(&self.config).await?);
        }
        Ok(self.clickhouse_client.as_ref().unwrap())
    }

    pub async fn cleanup(self) -> Result<()> {
        if let Some(redis_client) = self.redis_client {
            redis_helpers::clear_redis(&redis_client).await?;
        }
        Ok(())
    }
}

/// Macro for creating test scenarios
#[macro_export]
macro_rules! test_scenario {
    ($name:expr, $stages:expr) => {
        serde_json::json!({
            "slug": $name,
            "name": $name,
            "description": format!("Test scenario: {}", $name),
            "pipeline": {
                "stages": $stages
            },
            "priority": 100,
            "enabled": true,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339()
        })
    };
}

/// Macro for creating test pipeline stages
#[macro_export]
macro_rules! test_stage {
    ($type:expr, $params:expr) => {
        serde_json::json!({
            "type": $type,
            "params": $params
        })
    };
}

pub use test_scenario;
pub use test_stage;