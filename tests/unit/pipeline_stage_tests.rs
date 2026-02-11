//! Unit tests for pipeline stages
//!
//! Tests individual pipeline stages in isolation with mocked dependencies

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use mockall::predicate::*;
use mockall::mock;

use bongas_ai::pipeline::{PipelineStage, ScoredItem};
use bongas_ai::pipeline::context::ExecutionContext;
use bongas_ai::cache::redis::RedisClient as RedisClientTrait;
use bongas_ai::cache::postgres_cache::PostgresCache;
use bongas_ai::db::repositories::cache_repository::CacheRepository;
use bongas_ai::engine::staging_manager::StagingManager;

use crate::common::{
    TestConfig, setup_test_db, setup_test_redis, setup_test_staging_manager,
    fixtures::{create_test_items, create_test_items_with_metadata, create_test_context},
    db_helpers::seed_test_data,
    redis_helpers::clear_redis,
    clickhouse_helpers::{setup_test_tables, seed_test_data as seed_clickhouse_data},
    mocks::{MockClickHouseClient, MockRedisClient},
};

mod onnx_inference_tests {
    use super::*;

    #[tokio::test]
    async fn test_onnx_inference_stage_execution() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        // Seed test data
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create test items
        let input_items = create_test_items(5);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        // Test ONNX inference stage
        let onnx_stage = bongas_ai::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
        let params = serde_json::json!({
            "model_name": "two_tower_v1",
            "model_format": "onnx",
            "top_k": 3,
            "model_path": "models/two_tower_v1.onnx"
        });

        let result = onnx_stage.execute(&context, &params, input_items).await?;
        
        // Verify results
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|item| item.score >= 0.0));
        assert!(result.iter().all(|item| item.metadata.get("inference_engine") == Some(&JsonValue::String("onnx".to_string()))));
        assert!(result.iter().all(|item| item.metadata.get("model_name") == Some(&JsonValue::String("two_tower_v1".to_string()))));

        Ok(())
    }

    #[tokio::test]
    async fn test_onnx_inference_stage_no_user_id() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(3);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: None, // No user ID
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let onnx_stage = bongas_ai::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
        let params = serde_json::json!({
            "model_name": "two_tower_v1",
            "model_format": "onnx",
            "top_k": 3
        });

        // Should fail due to missing user_id
        let result = onnx_stage.execute(&context, &params, input_items).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("user_id required"));

        Ok(())
    }

    #[tokio::test]
    async fn test_onnx_inference_stage_top_k_limit() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(10);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let onnx_stage = bongas_ai::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
        let params = serde_json::json!({
            "model_name": "two_tower_v1",
            "model_format": "onnx",
            "top_k": 5
        });

        let result = onnx_stage.execute(&context, &params, input_items).await?;
        
        // Should be limited to top_k
        assert_eq!(result.len(), 5);
        assert!(result.iter().all(|item| item.score >= 0.0));

        Ok(())
    }
}

mod filter_watched_tests {
    use super::*;

    #[tokio::test]
    async fn test_filter_watched_stage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create items where some have been watched
        let input_items = vec![
            ScoredItem { item_id: 1, score: 0.9, metadata: JsonValue::Object(serde_json::Map::new()) }, // Watched
            ScoredItem { item_id: 2, score: 0.8, metadata: JsonValue::Object(serde_json::Map::new()) }, // Watched
            ScoredItem { item_id: 6, score: 0.7, metadata: JsonValue::Object(serde_json::Map::new()) }, // Not watched
            ScoredItem { item_id: 7, score: 0.6, metadata: JsonValue::Object(serde_json::Map::new()) }, // Not watched
        ];

        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let filter_stage = bongas_ai::pipeline::stages::filter::filter_already_watched::FilterWatchedStage;
        let params = serde_json::json!({
            "max_watched": 50
        });

        let result = filter_stage.execute(&context, &params, input_items).await?;
        
        // Should filter out watched items (1, 2)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].item_id, 6);
        assert_eq!(result[1].item_id, 7);
        assert!(result.iter().all(|item| item.metadata.get("filtered") == Some(&JsonValue::Bool(true))));

        Ok(())
    }

    #[tokio::test]
    async fn test_filter_watched_stage_no_user_id() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(3);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: None, // No user ID
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let filter_stage = bongas_ai::pipeline::stages::filter::filter_already_watched::FilterWatchedStage;
        let params = serde_json::json!({
            "max_watched": 50
        });

        // Should pass through all items when no user_id
        let result = filter_stage.execute(&context, &params, input_items).await?;
        
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|item| item.metadata.get("filtered") == Some(&JsonValue::Bool(false))));

        Ok(())
    }
}

mod diversify_genre_tests {
    use super::*;

    #[tokio::test]
    async fn test_diversify_by_genre_stage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create items with different genres
        let input_items = create_test_items_with_metadata(10);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let diversify_stage = bongas_ai::pipeline::stages::diversify::diversify_genres::DiversifyByGenreStage;
        let params = serde_json::json!({
            "max_same_genre": 2
        });

        let result = diversify_stage.execute(&context, &params, input_items).await?;
        
        // Count items by genre
        let mut genre_counts = std::collections::HashMap::new();
        for item in &result {
            if let Some(genre) = item.metadata.get("genre").and_then(|g| g.as_str()) {
                *genre_counts.entry(genre.to_string()).or_insert(0) += 1;
            }
        }

        // Verify no genre exceeds max_same_genre
        for (genre, count) in genre_counts {
            assert!(count <= 2, "Genre {} has {} items, exceeds limit of 2", genre, count);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_diversify_by_genre_stage_no_metadata() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(5);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let diversify_stage = bongas_ai::pipeline::stages::diversify::diversify_genres::DiversifyByGenreStage;
        let params = serde_json::json!({
            "max_same_genre": 2
        });

        // Should pass through all items when no genre metadata
        let result = diversify_stage.execute(&context, &params, input_items).await?;
        
        assert_eq!(result.len(), 5);
        assert!(result.iter().all(|item| item.metadata.get("diversified") == Some(&JsonValue::Bool(true))));

        Ok(())
    }
}

mod fetch_trending_tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_clickhouse_trending_stage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let trending_stage = bongas_ai::pipeline::stages::fetch::fetch_clickhouse_trending::ClickHouseTrendingStage;
        let params = serde_json::json!({
            "limit": 10,
            "time_window": "1h",
            "min_interactions": 5
        });

        let result = trending_stage.execute(&context, &params, vec![]).await?;
        
        // Should return trending items
        assert!(!result.is_empty());
        assert!(result.len() <= 10);
        assert!(result.iter().all(|item| item.score >= 0.0));
        assert!(result.iter().all(|item| item.metadata.get("source") == Some(&JsonValue::String("clickhouse_trending".to_string()))));

        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_clickhouse_trending_stage_with_clickhouse() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        let clickhouse_client = bongas_ai::common::clickhouse::ClickHouseClient::new(&config.clickhouse_url)?;
        
        seed_test_data(&db_pool).await?;
        setup_test_tables(&clickhouse_client).await?;
        seed_clickhouse_data(&clickhouse_client).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: Some(clickhouse_client.clone()),
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let trending_stage = bongas_ai::pipeline::stages::fetch::fetch_clickhouse_trending::ClickHouseTrendingStage;
        let params = serde_json::json!({
            "limit": 5,
            "time_window": "1h",
            "min_interactions": 1
        });

        let result = trending_stage.execute(&context, &params, vec![]).await?;
        
        // Should return items from ClickHouse
        assert!(!result.is_empty());
        assert!(result.len() <= 5);

        Ok(())
    }
}

mod sort_tests {
    use super::*;

    #[tokio::test]
    async fn test_sort_by_score_stage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create items with different scores
        let mut input_items = create_test_items(5);
        // Manually set scores to ensure they're not in order
        input_items[0].score = 0.3;
        input_items[1].score = 0.9;
        input_items[2].score = 0.1;
        input_items[3].score = 0.7;
        input_items[4].score = 0.5;

        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let sort_stage = bongas_ai::pipeline::stages::sort::sort_by_score::SortByScoreStage;
        let params = serde_json::json!({
            "descending": true
        });

        let result = sort_stage.execute(&context, &params, input_items).await?;
        
        // Should be sorted by score descending
        assert_eq!(result.len(), 5);
        for i in 1..result.len() {
            assert!(result[i-1].score >= result[i].score);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_limit_stage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(10);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let limit_stage = bongas_ai::pipeline::stages::sort::limit::LimitStage;
        let params = serde_json::json!({
            "limit": 5
        });

        let result = limit_stage.execute(&context, &params, input_items).await?;
        
        // Should be limited to 5 items
        assert_eq!(result.len(), 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_deduplicate_stage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create items with duplicates
        let input_items = vec![
            ScoredItem { item_id: 1, score: 0.9, metadata: JsonValue::Object(serde_json::Map::new()) },
            ScoredItem { item_id: 2, score: 0.8, metadata: JsonValue::Object(serde_json::Map::new()) },
            ScoredItem { item_id: 1, score: 0.7, metadata: JsonValue::Object(serde_json::Map::new()) }, // Duplicate
            ScoredItem { item_id: 3, score: 0.6, metadata: JsonValue::Object(serde_json::Map::new()) },
            ScoredItem { item_id: 2, score: 0.5, metadata: JsonValue::Object(serde_json::Map::new()) }, // Duplicate
        ];

        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let dedupe_stage = bongas_ai::pipeline::stages::sort::deduplicate::DeduplicateStage;
        let params = serde_json::json!({});

        let result = dedupe_stage.execute(&context, &params, input_items).await?;
        
        // Should remove duplicates, keeping first occurrence
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].item_id, 1);
        assert_eq!(result[1].item_id, 2);
        assert_eq!(result[2].item_id, 3);

        Ok(())
    }
}

mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_stage_error_handling() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(3);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        // Test with invalid JSON parameters
        let onnx_stage = bongas_ai::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
        let invalid_params = serde_json::json!({
            "invalid_param": "invalid_value"
        });

        let result = onnx_stage.execute(&context, &invalid_params, input_items).await;
        
        // Should handle parameter parsing errors gracefully
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_stage_with_missing_dependencies() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let input_items = create_test_items(3);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: None, // Missing database pool
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let onnx_stage = bongas_ai::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
        let params = serde_json::json!({
            "model_name": "two_tower_v1",
            "model_format": "onnx",
            "top_k": 3
        });

        let result = onnx_stage.execute(&context, &params, input_items).await;
        
        // Should handle missing database gracefully
        assert!(result.is_err());

        Ok(())
    }
}

mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_stage_performance() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create large input for performance testing
        let input_items = create_test_items(1000);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let sort_stage = bongas_ai::pipeline::stages::sort::sort_by_score::SortByScoreStage;
        let params = serde_json::json!({
            "descending": true
        });

        let start = std::time::Instant::now();
        let result = sort_stage.execute(&context, &params, input_items).await?;
        let duration = start.elapsed();

        // Should complete within reasonable time (adjust based on your requirements)
        assert!(duration.as_millis() < 1000, "Stage took too long: {:?}", duration);
        assert_eq!(result.len(), 1000);

        Ok(())
    }

    #[tokio::test]
    async fn test_stage_memory_usage() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Test with very large input
        let input_items = create_test_items(10000);
        let context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: create_test_context(),
        };

        let limit_stage = bongas_ai::pipeline::stages::sort::limit::LimitStage;
        let params = serde_json::json!({
            "limit": 100
        });

        let result = limit_stage.execute(&context, &params, input_items).await?;
        
        // Should limit memory usage by only returning limited results
        assert_eq!(result.len(), 100);
        assert!(result.capacity() <= 1000); // Should not hold onto all input

        Ok(())
    }
}