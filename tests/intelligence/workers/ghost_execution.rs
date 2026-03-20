use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use bongas_ai::engine::intelligence::workers::ghost_execution::service::GhostExecutionWorker;
use bongas_ai::cache::{CacheManager, CacheConfig};
use bongas_ai::ml::inference::onnx::service::OnnxInferenceEngine;
use bongas_ai::ingestion::UserActivity;
use bongas_ai::config::RedisConfig;
use bongas_ai::circuit_breaker::observer::CircuitBreakerId;

#[tokio::test]
async fn test_ghost_execution_flow() {
    // 1. Setup Cache Manager (L1 only for testing)
    let cache_config = CacheConfig {
        l1_enabled: true,
        l2_enabled: false,
        ..Default::default()
    };
    
    let redis_config = RedisConfig::default();

    let cache_manager = Arc::new(CacheManager::new(redis_config, cache_config, None).await.unwrap());

    // 2. Setup Mock ONNX Engine
    let onnx_engine = Arc::new(OnnxInferenceEngine::with_defaults(
        CircuitBreakerId::new("ml.inference.flow_head")
    ));

    // 3. Setup Worker
    let worker = GhostExecutionWorker::new(cache_manager.clone(), onnx_engine);
    let notifier = worker.get_notifier();
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let worker_clone = worker.clone();
    tokio::spawn(async move {
        worker_clone.start(shutdown_rx).await;
    });

    // 4. Simulate User Activity (3 events to trigger prediction)
    let profile_id = "test_user_123";
    
    for i in 1..=3 {
        let activity = UserActivity::Playback {
            user_id: 1,
            profile_id: Some(profile_id.to_string()),
            item_id: 100 + i,
            session_id: "session_1".to_string(),
            visitor_id: Some("vid".to_string()),
            device_hash: Some("hash".to_string()),
            device_type: Some("mobile".to_string()),
            watch_duration_seconds: 60,
            total_duration_seconds: 120,
            watch_percentage: 0.5,
            completed: false,
            scenario_slug: Some("home".to_string()),
            timestamp: chrono::Utc::now(),
        };
        notifier.send(activity).await.unwrap();
        sleep(Duration::from_millis(50)).await;
    }

    // 5. Verify History in L1 Cache
    let history_key = format!("user:history:{}", profile_id);
    let history = cache_manager.get_list(&history_key).await.unwrap();
    assert!(history.len() >= 3);
    assert_eq!(history[0], "103");
    assert_eq!(history[1], "102");
    assert_eq!(history[2], "101");

    // 6. Verify Ghost Cache (Predictions)
    let ghost_key = format!("ghost:cache:{}", profile_id);
    
    let mut found = false;
    for _ in 0..20 {
        if let Some(val) = cache_manager.get_raw(&ghost_key).await.unwrap() {
            assert_eq!(val, "104,105,106");
            found = true;
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    assert!(found, "Ghost Cache should have been populated with predictions");

    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_ghost_execution_anonymous_skip() {
    let cache_config = CacheConfig { l1_enabled: true, l2_enabled: false, ..Default::default() };
    let cache_manager = Arc::new(CacheManager::new(RedisConfig::default(), cache_config, None).await.unwrap());
    let onnx_engine = Arc::new(OnnxInferenceEngine::with_defaults(CircuitBreakerId::new("test")));
    let worker = GhostExecutionWorker::new(cache_manager.clone(), onnx_engine);
    let notifier = worker.get_notifier();
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let worker_clone = worker.clone();
    tokio::spawn(async move {
        worker_clone.start(shutdown_rx).await;
    });

    let activity = UserActivity::Playback {
        user_id: 0,
        profile_id: None,
        item_id: 101,
        session_id: "anon".to_string(),
        visitor_id: Some("vid".to_string()),
        device_hash: Some("hash".to_string()),
        device_type: Some("mobile".to_string()),
        watch_duration_seconds: 10,
        total_duration_seconds: 100,
        watch_percentage: 0.1,
        completed: false,
        scenario_slug: Some("home".to_string()),
        timestamp: chrono::Utc::now(),
    };
    notifier.send(activity).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let history_key = "user:history:None"; 
    let history = cache_manager.get_list(history_key).await.unwrap();
    assert!(history.is_empty());

    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_ghost_execution_short_history_skip() {
    let cache_config = CacheConfig { l1_enabled: true, l2_enabled: false, ..Default::default() };
    let cache_manager = Arc::new(CacheManager::new(RedisConfig::default(), cache_config, None).await.unwrap());
    let onnx_engine = Arc::new(OnnxInferenceEngine::with_defaults(CircuitBreakerId::new("test")));
    let worker = GhostExecutionWorker::new(cache_manager.clone(), onnx_engine);
    let notifier = worker.get_notifier();
    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let worker_clone = worker.clone();
    tokio::spawn(async move {
        worker_clone.start(shutdown_rx).await;
    });

    let profile_id = "short_user";
    
    let activity = UserActivity::Playback {
        user_id: 1,
        profile_id: Some(profile_id.to_string()),
        item_id: 101,
        session_id: "sess".to_string(),
        visitor_id: Some("vid".to_string()),
        device_hash: Some("hash".to_string()),
        device_type: Some("mobile".to_string()),
        watch_duration_seconds: 60,
        total_duration_seconds: 120,
        watch_percentage: 0.5,
        completed: false,
        scenario_slug: Some("home".to_string()),
        timestamp: chrono::Utc::now(),
    };
    notifier.send(activity).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let history_key = format!("user:history:{}", profile_id);
    let history = cache_manager.get_list(&history_key).await.unwrap();
    assert_eq!(history.len(), 1);

    let ghost_key = format!("ghost:cache:{}", profile_id);
    let ghost = cache_manager.get_raw(&ghost_key).await.unwrap();
    assert!(ghost.is_none());

    let _ = shutdown_tx.send(());
}
