//! Performance benchmarks for BONGAS-AI pipeline stages
//!
//! Uses Criterion for comprehensive performance testing and benchmarking

use std::sync::Arc;
use std::time::Duration;
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use serde_json::Value as JsonValue;

// Mock implementations for benchmarking
#[derive(Debug, Clone)]
pub struct ScoredItem {
    pub item_id: i64,
    pub score: f32,
    pub metadata: JsonValue,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub request_id: String,
    pub user_id: Option<i32>,
    pub context: JsonValue,
}

impl ScoredItem {
    pub fn new(item_id: i64, score: f32) -> Self {
        Self {
            item_id,
            score,
            metadata: JsonValue::Object(serde_json::Map::new()),
        }
    }
}

impl ExecutionContext {
    pub fn new(user_id: i32) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            user_id: Some(user_id),
            context: serde_json::json!({
                "device_type": "mobile",
                "location": "kenya"
            }),
        }
    }
}

/// Benchmark pipeline stage execution times
fn bench_pipeline_stages(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    // Setup test environment
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("pipeline_stages");

    // Benchmark different input sizes
    for item_count in [10, 50, 100, 500, 1000, 2000] {
        let input_items = create_test_items(item_count);
        let context = create_test_context();
        
        // Create execution context
        let exec_context = ExecutionContext {
            request_id: uuid::Uuid::new_v4(),
            user_id: Some(123),
            db_pool: Some(db_pool.clone()),
            redis_client: Some(redis_client.clone()),
            clickhouse_client: None,
            staging_manager: Some(staging_manager.clone()),
            context: context.clone(),
        };

        // Benchmark ONNX inference stage
        group.bench_with_input(
            BenchmarkId::new("onnx_inference", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let onnx_stage = bongas_ai::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
                    let params = serde_json::json!({
                        "model_name": "two_tower_v1",
                        "model_format": "onnx",
                        "top_k": 10
                    });
                    
                    // Clone items for each iteration
                    let test_items = create_test_items(item_count);
                    onnx_stage.execute(&exec_context, &params, test_items).await.unwrap()
                })
            },
        );

        // Benchmark filter watched stage
        group.bench_with_input(
            BenchmarkId::new("filter_watched", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let filter_stage = bongas_ai::pipeline::stages::filter::filter_already_watched::FilterWatchedStage;
                    let params = serde_json::json!({
                        "max_watched": 50
                    });
                    
                    let test_items = create_test_items(item_count);
                    filter_stage.execute(&exec_context, &params, test_items).await.unwrap()
                })
            },
        );

        // Benchmark diversify by genre stage
        group.bench_with_input(
            BenchmarkId::new("diversify_genre", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let diversify_stage = bongas_ai::pipeline::stages::diversify::diversify_genres::DiversifyByGenreStage;
                    let params = serde_json::json!({
                        "max_same_genre": 3
                    });
                    
                    let test_items = create_test_items_with_metadata(item_count);
                    diversify_stage.execute(&exec_context, &params, test_items).await.unwrap()
                })
            },
        );

        // Benchmark sort by score stage
        group.bench_with_input(
            BenchmarkId::new("sort_by_score", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let sort_stage = bongas_ai::pipeline::stages::sort::sort_by_score::SortByScoreStage;
                    let params = serde_json::json!({
                        "descending": true
                    });
                    
                    let test_items = create_test_items(item_count);
                    sort_stage.execute(&exec_context, &params, test_items).await.unwrap()
                })
            },
        );

        // Benchmark limit stage
        group.bench_with_input(
            BenchmarkId::new("limit", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let limit_stage = bongas_ai::pipeline::stages::sort::limit::LimitStage;
                    let params = serde_json::json!({
                        "limit": 100
                    });
                    
                    let test_items = create_test_items(item_count);
                    limit_stage.execute(&exec_context, &params, test_items).await.unwrap()
                })
            },
        );
    }

    group.finish();
}

/// Benchmark cache operations
fn bench_cache_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("cache_operations");

    // Benchmark cache save operations
    for item_count in [10, 50, 100, 500] {
        let items = create_test_items(item_count);
        
        group.bench_with_input(
            BenchmarkId::new("cache_save", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let test_items = create_test_items(item_count);
                    staging_manager.save_cached("test_scenario", Some(123), "test_context", &test_items, 300).await.unwrap()
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cache_get", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    staging_manager.get_cached("test_scenario", Some(123), "test_context").await.unwrap()
                })
            },
        );
    }

    group.finish();
}

/// Benchmark ML components
fn bench_ml_components(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("ml_components");

    // Benchmark feature extraction
    for user_count in [1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("feature_extraction", user_count),
            &user_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let feature_store = bongas_ai::ml::feature_store::FeatureStore::new(
                        db_pool.clone(),
                        redis_client.clone(),
                    );
                    
                    for i in 0..user_count {
                        feature_store.get_user_features(123 + i as i32).await.unwrap();
                    }
                })
            },
        );
    }

    // Benchmark bandit algorithms
    group.bench_function("thompson_sampling_selection", |b| {
        b.iter(|| {
            let mut ts = bongas_ai::ml::bandits::ThompsonSampling::new(10);
            ts.select_arm()
        })
    });

    group.bench_function("thompson_sampling_update", |b| {
        b.iter(|| {
            let mut ts = bongas_ai::ml::bandits::ThompsonSampling::new(10);
            ts.update(5, 1.0)
        })
    });

    group.finish();
}

/// Benchmark pipeline execution end-to-end
fn bench_end_to_end_pipeline(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("end_to_end_pipeline");

    // Benchmark complete pipeline execution
    for scenario_type in ["trending_now", "for_you_personalized", "continue_watching"] {
        for user_count in [1, 10, 100] {
            group.bench_with_input(
                BenchmarkId::new(format!("scenario_{}", scenario_type), user_count),
                &user_count,
                |b, &_count| {
                    b.to_async(&rt).iter(|| async {
                        let engine = bongas_ai::engine::BongasEngine::new(
                            db_pool.clone(),
                            redis_client.clone(),
                            staging_manager.clone(),
                            None,
                        );
                        
                        for i in 0..user_count {
                            engine.execute_scenario(scenario_type, 123 + i as i32, create_test_context()).await.unwrap();
                        }
                    })
                },
            );
        }
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");

    // Benchmark concurrent pipeline executions
    for concurrency in [1, 5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_pipeline", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let engine = bongas_ai::engine::BongasEngine::new(
                        db_pool.clone(),
                        redis_client.clone(),
                        staging_manager.clone(),
                        None,
                    );
                    
                    let handles: Vec<_> = (0..concurrency).map(|i| {
                        let engine = engine.clone();
                        tokio::spawn(async move {
                            engine.execute_scenario("trending_now", 123 + i, create_test_context()).await.unwrap()
                        })
                    }).collect();
                    
                    for handle in handles {
                        handle.await.unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("memory_usage");

    // Benchmark memory usage with large item lists
    for item_count in [1000, 5000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("large_item_list", item_count),
            &item_count,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let items = create_test_items(item_count);
                    let context = create_test_context();
                    
                    let exec_context = ExecutionContext {
                        request_id: uuid::Uuid::new_v4(),
                        user_id: Some(123),
                        db_pool: Some(db_pool.clone()),
                        redis_client: Some(redis_client.clone()),
                        clickhouse_client: None,
                        staging_manager: Some(staging_manager.clone()),
                        context: context.clone(),
                    };

                    let sort_stage = bongas_ai::pipeline::stages::sort::sort_by_score::SortByScoreStage;
                    let params = serde_json::json!({
                        "descending": true
                    });
                    
                    sort_stage.execute(&exec_context, &params, items).await.unwrap()
                })
            },
        );
    }

    group.finish();
}

/// Benchmark cache hit rates and performance
fn bench_cache_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = TestConfig::new();
    let db_pool = rt.block_on(setup_test_db(&config)).unwrap();
    let redis_client = rt.block_on(setup_test_redis(&config)).unwrap();
    
    rt.block_on(seed_test_data(&db_pool)).unwrap();
    rt.block_on(clear_redis(&redis_client)).unwrap();

    let staging_manager = rt.block_on(setup_test_staging_manager(db_pool.clone(), redis_client.clone())).unwrap();
    
    let mut group = c.benchmark_group("cache_performance");

    // Benchmark cache warming
    group.bench_function("cache_warming", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = bongas_ai::engine::BongasEngine::new(
                db_pool.clone(),
                redis_client.clone(),
                staging_manager.clone(),
                None,
            );
            
            engine.warm_cache().await.unwrap()
        })
    });

    // Benchmark cache hit rate scenarios
    group.bench_function("cache_hit_scenario", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = bongas_ai::engine::BongasEngine::new(
                db_pool.clone(),
                redis_client.clone(),
                staging_manager.clone(),
                None,
            );
            
            // First call (cache miss)
            engine.execute_scenario("trending_now", 123, create_test_context()).await.unwrap();
            
            // Second call (cache hit)
            engine.execute_scenario("trending_now", 123, create_test_context()).await.unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    name = pipeline_benches;
    config = Criterion::default()
        .sample_size(20)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5))
        .with_plots();
    targets = 
        bench_pipeline_stages,
        bench_cache_operations,
        bench_ml_components,
        bench_end_to_end_pipeline,
        bench_concurrent_operations,
        bench_memory_usage,
        bench_cache_performance
);

criterion_main!(pipeline_benches);