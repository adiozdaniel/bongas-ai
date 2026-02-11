//! API integration tests
//!
//! Tests REST API endpoints, scenario CRUD operations,
//! recommendation endpoints, and authentication

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use axum::{
    http::{Request, StatusCode, HeaderValue},
    body::Body,
};
use tower::ServiceExt;
use uuid::Uuid;

use bongas_ai::api::create_app;
use bongas_ai::common::{
    TestConfig, setup_test_db, setup_test_redis, setup_test_staging_manager,
    fixtures::{create_test_context, create_test_scenario_config},
    db_helpers::{seed_test_data, cleanup_test_data},
    redis_helpers::clear_redis,
};

mod api_app_tests {
    use super::*;

    #[tokio::test]
    async fn test_api_app_creation() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        // Create API app
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None, // No ClickHouse for this test
        );

        // Test health check endpoint
        let request = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_metrics_endpoint() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test metrics endpoint
        let request = Request::builder()
            .uri("/metrics")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_docs_endpoint() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test OpenAPI docs endpoint
        let request = Request::builder()
            .uri("/docs")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }
}

mod scenario_crud_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Create scenario request
        let scenario_data = serde_json::json!({
            "slug": "test_create_scenario",
            "name": "Test Create Scenario",
            "description": "Test scenario for creation",
            "pipeline": {
                "stages": [
                    {
                        "type": "clickhouse_trending",
                        "params": {
                            "limit": 10
                        }
                    }
                ]
            },
            "priority": 100,
            "enabled": true
        });

        let request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(scenario_data.to_string()))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::CREATED);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get existing scenario
        let request = Request::builder()
            .uri("/api/v1/scenarios/trending_now")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get nonexistent scenario
        let request = Request::builder()
            .uri("/api/v1/scenarios/nonexistent")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Update scenario request
        let update_data = serde_json::json!({
            "name": "Updated Test Scenario",
            "description": "Updated description",
            "priority": 200,
            "enabled": false
        });

        let request = Request::builder()
            .uri("/api/v1/scenarios/trending_now")
            .method("PUT")
            .header("Content-Type", "application/json")
            .body(Body::from(update_data.to_string()))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // First create a scenario to delete
        let scenario_data = serde_json::json!({
            "slug": "test_delete_scenario",
            "name": "Test Delete Scenario",
            "description": "Test scenario for deletion",
            "pipeline": {
                "stages": [
                    {
                        "type": "clickhouse_trending",
                        "params": {
                            "limit": 5
                        }
                    }
                ]
            },
            "priority": 50,
            "enabled": true
        });

        let create_request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(scenario_data.to_string()))?;

        let create_response = app.clone().oneshot(create_request).await?;
        assert_eq!(create_response.status(), StatusCode::CREATED);

        // Now delete it
        let delete_request = Request::builder()
            .uri("/api/v1/scenarios/test_delete_scenario")
            .method("DELETE")
            .body(Body::empty())?;

        let delete_response = app.clone().oneshot(delete_request).await?;
        assert_eq!(delete_response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_scenarios() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // List scenarios
        let request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_scenario_validation() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test invalid scenario data
        let invalid_data = serde_json::json!({
            "name": "", // Empty name should be invalid
            "pipeline": {
                "stages": [] // Empty stages should be invalid
            }
        });

        let request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(invalid_data.to_string()))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}

mod recommendation_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_recommendations() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get recommendations for existing scenario
        let request = Request::builder()
            .uri("/api/v1/recommendations/trending_now/123")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_recommendations_with_context() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get recommendations with context
        let context = create_test_context();
        let request_body = serde_json::json!({
            "context": context
        });

        let request = Request::builder()
            .uri("/api/v1/recommendations/trending_now/123")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(request_body.to_string()))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_recommendations_nonexistent_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get recommendations for nonexistent scenario
        let request = Request::builder()
            .uri("/api/v1/recommendations/nonexistent/123")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_recommendations_invalid_user_id() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get recommendations with invalid user ID
        let request = Request::builder()
            .uri("/api/v1/recommendations/trending_now/invalid")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_recommendations_with_filters() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get recommendations with filters
        let filters = serde_json::json!({
            "filters": {
                "min_score": 0.5,
                "max_items": 10
            }
        });

        let request = Request::builder()
            .uri("/api/v1/recommendations/trending_now/123")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(filters.to_string()))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }
}

mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_stats_endpoint() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Get cache stats
        let request = Request::builder()
            .uri("/api/v1/admin/cache-stats")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_invalidation_endpoint() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Invalidate cache
        let request = Request::builder()
            .uri("/api/v1/admin/invalidate-cache")
            .method("POST")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_invalidation_by_scenario() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Invalidate cache for specific scenario
        let request = Request::builder()
            .uri("/api/v1/admin/invalidate-cache/trending_now")
            .method("POST")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        Ok(())
    }
}

mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_404_handling() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test 404 for non-existent endpoint
        let request = Request::builder()
            .uri("/api/v1/nonexistent")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_method_not_allowed() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test method not allowed
        let request = Request::builder()
            .uri("/api/v1/scenarios/trending_now")
            .method("PATCH") // PATCH not supported
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        Ok(())
    }

    #[tokio::test]
    async fn test_content_type_validation() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test invalid content type
        let request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("POST")
            .header("Content-Type", "text/plain") // Wrong content type
            .body(Body::from("invalid json"))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        Ok(())
    }

    #[tokio::test]
    async fn test_json_parsing_error() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Test invalid JSON
        let request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from("invalid json {"))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }
}

mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_api_response_time() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Measure response time
        let start = std::time::Instant::now();

        let request = Request::builder()
            .uri("/api/v1/recommendations/trending_now/123")
            .method("GET")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        
        let duration = start.elapsed();
        
        assert_eq!(response.status(), StatusCode::OK);
        assert!(duration.as_millis() < 1000, "API response too slow: {:?}", duration);

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_requests() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        seed_test_data(&db_pool).await?;
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Send multiple concurrent requests
        let handles: Vec<_> = (0..10).map(|i| {
            let app = app.clone();
            tokio::spawn(async move {
                let request = Request::builder()
                    .uri(&format!("/api/v1/recommendations/trending_now/{}", 123 + i))
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                app.oneshot(request).await
            })
        }).collect();

        // Wait for all requests to complete
        for handle in handles {
            let response = handle.await?;
            assert_eq!(response.status(), StatusCode::OK);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_large_payload_handling() -> Result<()> {
        let config = TestConfig::new();
        let db_pool = setup_test_db(&config).await?;
        let redis_client = setup_test_redis(&config).await?;
        
        clear_redis(&redis_client).await?;

        let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
        
        let app = create_app(
            db_pool,
            redis_client,
            staging_manager,
            None,
        );

        // Create large scenario configuration
        let large_pipeline = serde_json::json!({
            "stages": (0..100).map(|i| {
                json!({
                    "type": "test_stage",
                    "params": {
                        "stage_id": i,
                        "config": (0..10).map(|j| format!("config_{}_{}", i, j)).collect::<Vec<_>>()
                    }
                })
            }).collect::<Vec<_>>()
        });

        let large_scenario = serde_json::json!({
            "slug": "large_scenario",
            "name": "Large Test Scenario",
            "description": "Test scenario with large configuration",
            "pipeline": large_pipeline,
            "priority": 100,
            "enabled": true
        });

        let request = Request::builder()
            .uri("/api/v1/scenarios")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(large_scenario.to_string()))?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::CREATED);

        Ok(())
    }
}