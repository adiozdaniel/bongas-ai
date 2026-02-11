use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

use bongas_ai::cache::redis::RedisClient as BongasRedisClient;
use bongas_ai::cache::postgres_cache::PostgresCache;
use bongas_ai::db::repositories::cache_repository::CacheRepository;
use bongas_ai::engine::staging_manager::{StagingManager, StagingStats};
use bongas_ai::pipeline::ScoredItem;
use bongas_ai::common::{
    TestConfig, setup_test_db, setup_test_redis, setup_test_staging_manager,
    fixtures::create_test_items, db_helpers::seed_test_data,
    redis_helpers::clear_redis,
};

#[tokio::test]
async fn test_l1_cache_hit() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save to L1 (Redis)
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 300).await?;

    // Retrieve from L1
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_some());
    let cached_items = cached.unwrap();
    assert_eq!(cached_items.len(), items.len());
    
    // Verify items are the same
    for (i, item) in items.iter().enumerate() {
        assert_eq!(cached_items[i].item_id, item.item_id);
        assert_eq!(cached_items[i].score, item.score);
    }

    Ok(())
}

#[tokio::test]
async fn test_l2_cache_fallback() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save to L2 (Postgres) only
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 300).await?;

    // L1 miss (should promote to L1 from L2)
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_some());
    let cached_items = cached.unwrap();
    assert_eq!(cached_items.len(), items.len());

    Ok(())
}

#[tokio::test]
async fn test_cache_invalidation() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save to cache
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 300).await?;

    // Verify cache hit
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_some());

    // Invalidate
    staging_manager.invalidate("test_scenario", 123).await?;

    // Cache should be invalidated
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_none());

    Ok(())
}

#[tokio::test]
async fn test_profile_invalidation() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save to cache for user 123
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 300).await?;
    staging_manager.save_cached("another_scenario", Some(123), "test_context", &items, 300).await?;

    // Verify cache hits
    let cached1 = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    let cached2 = staging_manager.get_cached("another_scenario", Some(123), "test_context").await?;
    assert!(cached1.is_some());
    assert!(cached2.is_some());

    // Invalidate all caches for profile
    staging_manager.invalidate_profile(123).await?;

    // Both caches should be invalidated
    let cached1 = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    let cached2 = staging_manager.get_cached("another_scenario", Some(123), "test_context").await?;
    assert!(cached1.is_none());
    assert!(cached2.is_none());

    Ok(())
}

#[tokio::test]
async fn test_cache_metrics() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Initial stats
    let initial_stats = staging_manager.get_stats();
    assert_eq!(initial_stats.l1_hits, 0);
    assert_eq!(initial_stats.l1_misses, 0);
    assert_eq!(initial_stats.l2_hits, 0);
    assert_eq!(initial_stats.l2_misses, 0);
    assert_eq!(initial_stats.invalidations, 0);

    // Simulate L1 miss, L2 hit
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 300).await?;
    let _ = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;

    // Check stats
    let stats = staging_manager.get_stats();
    assert_eq!(stats.l1_misses, 1);
    assert_eq!(stats.l2_hits, 1);
    assert!(stats.hit_rate > 0.0);

    // Simulate cache invalidation
    staging_manager.invalidate("test_scenario", 123).await?;
    let stats = staging_manager.get_stats();
    assert_eq!(stats.invalidations, 1);

    Ok(())
}

#[tokio::test]
async fn test_context_hashing() -> Result<()> {
    let context1 = serde_json::json!({"device_type": "mobile", "location": "kenya"});
    let context2 = serde_json::json!({"device_type": "desktop", "location": "kenya"});
    let context3 = serde_json::json!({"device_type": "mobile", "location": "kenya"});

    let hash1 = StagingManager::hash_context(&context1);
    let hash2 = StagingManager::hash_context(&context2);
    let hash3 = StagingManager::hash_context(&context3);

    // Same context should produce same hash
    assert_eq!(hash1, hash3);
    
    // Different context should produce different hash
    assert_ne!(hash1, hash2);

    Ok(())
}

#[tokio::test]
async fn test_anonymous_user_caching() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save to cache for anonymous user (None)
    staging_manager.save_cached("test_scenario", None, "test_context", &items, 300).await?;

    // Retrieve from cache for anonymous user
    let cached = staging_manager.get_cached("test_scenario", None, "test_context").await?;
    assert!(cached.is_some());

    // Should not match when user_id is provided
    let cached_with_user = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached_with_user.is_none());

    Ok(())
}

#[tokio::test]
async fn test_cache_ttl_expiration() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save with short TTL
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 1).await?;

    // Should be cached initially
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_some());

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should be expired
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_none());

    Ok(())
}

#[tokio::test]
async fn test_cache_promotion_from_l2_to_l1() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save only to L2 (Postgres)
    staging_manager.save_cached("test_scenario", Some(123), "test_context", &items, 300).await?;

    // Clear L1 to simulate miss
    staging_manager.invalidate("test_scenario", 123).await?;

    // This should promote from L2 to L1
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_some());

    // Verify it's now in L1
    let cached = staging_manager.get_cached("test_scenario", Some(123), "test_context").await?;
    assert!(cached.is_some());

    Ok(())
}

#[tokio::test]
async fn test_cache_scenario_specific() -> Result<()> {
    let config = TestConfig::new();
    let db_pool = setup_test_db(&config).await?;
    let redis_client = setup_test_redis(&config).await?;
    
    seed_test_data(&db_pool).await?;
    clear_redis(&redis_client).await?;

    let staging_manager = setup_test_staging_manager(db_pool.clone(), redis_client.clone()).await?;
    let items = create_test_items();

    // Save different items for different scenarios
    let items1 = create_test_items();
    let items2 = create_test_items().into_iter().map(|mut item| { item.item_id += 100; item }).collect();

    staging_manager.save_cached("scenario1", Some(123), "test_context", &items1, 300).await?;
    staging_manager.save_cached("scenario2", Some(123), "test_context", &items2, 300).await?;

    // Should retrieve correct items for each scenario
    let cached1 = staging_manager.get_cached("scenario1", Some(123), "test_context").await?;
    let cached2 = staging_manager.get_cached("scenario2", Some(123), "test_context").await?;

    assert!(cached1.is_some());
    assert!(cached2.is_some());

    let cached1_items = cached1.unwrap();
    let cached2_items = cached2.unwrap();

    assert_eq!(cached1_items[0].item_id, 1); // Original items
    assert_eq!(cached2_items[0].item_id, 101); // Modified items

    Ok(())
}
