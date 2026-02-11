//! Simple performance benchmarks for BONGAS-AI
//!
//! Uses Criterion for basic performance testing

use std::time::Duration;
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

// Mock data structures for benchmarking
#[derive(Debug, Clone)]
struct ScoredItem {
    item_id: i64,
    score: f32,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
struct PipelineContext {
    user_id: i32,
    device_type: String,
    location: String,
}

impl ScoredItem {
    fn new(item_id: i64, score: f32) -> Self {
        Self {
            item_id,
            score,
            metadata: serde_json::json!({}),
        }
    }
}

impl PipelineContext {
    fn new(user_id: i32) -> Self {
        Self {
            user_id,
            device_type: "mobile".to_string(),
            location: "kenya".to_string(),
        }
    }
}

// Mock pipeline stages for benchmarking
struct MockPipelineStages;

impl MockPipelineStages {
    // Simulate ONNX inference stage
    fn onnx_inference_stage(items: Vec<ScoredItem>, context: &PipelineContext) -> Vec<ScoredItem> {
        items.into_iter()
            .map(|mut item| {
                // Simulate scoring based on user context
                let base_score = item.score;
                let context_boost = if context.device_type == "mobile" { 0.1 } else { 0.05 };
                item.score = base_score + context_boost;
                item
            })
            .collect()
    }

    // Simulate filter watched stage
    fn filter_watched_stage(items: Vec<ScoredItem>, watched_items: &[i64]) -> Vec<ScoredItem> {
        items.into_iter()
            .filter(|item| !watched_items.contains(&item.item_id))
            .collect()
    }

    // Simulate diversify by genre stage
    fn diversify_by_genre_stage(items: Vec<ScoredItem>, max_same_genre: usize) -> Vec<ScoredItem> {
        let mut genre_counts = std::collections::HashMap::new();
        let mut result = Vec::new();

        for item in items {
            let genre = item.metadata.get("genre")
                .and_then(|g| g.as_str())
                .unwrap_or("unknown");

            let count = genre_counts.entry(genre.to_string()).or_insert(0);
            if *count < max_same_genre {
                *count += 1;
                result.push(item);
            }
        }

        result
    }

    // Simulate sort by score stage
    fn sort_by_score_stage(items: Vec<ScoredItem>, descending: bool) -> Vec<ScoredItem> {
        let mut sorted = items;
        if descending {
            sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        } else {
            sorted.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        }
        sorted
    }

    // Simulate limit stage
    fn limit_stage(items: Vec<ScoredItem>, limit: usize) -> Vec<ScoredItem> {
        items.into_iter().take(limit).collect()
    }
}

// Benchmark pipeline stage execution times
fn bench_pipeline_stages(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_stages");

    // Benchmark different input sizes
    for item_count in [10, 50, 100, 500, 1000, 2000] {
        let input_items: Vec<ScoredItem> = (0..item_count)
            .map(|i| ScoredItem::new(i as i64, (i as f32) / 100.0))
            .collect();
        let context = PipelineContext::new(123);

        // Benchmark ONNX inference stage
        group.bench_with_input(
            BenchmarkId::new("onnx_inference", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    MockPipelineStages::onnx_inference_stage(input_items.clone(), &context)
                })
            },
        );

        // Benchmark filter watched stage
        let watched_items = vec![1, 3, 5, 7, 9];
        group.bench_with_input(
            BenchmarkId::new("filter_watched", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    MockPipelineStages::filter_watched_stage(input_items.clone(), &watched_items)
                })
            },
        );

        // Benchmark diversify by genre stage
        let items_with_genres: Vec<ScoredItem> = input_items.iter()
            .enumerate()
            .map(|(i, item)| {
                let mut item = item.clone();
                item.metadata = serde_json::json!({
                    "genre": if i % 3 == 0 { "Action" } else if i % 3 == 1 { "Drama" } else { "Comedy" }
                });
                item
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("diversify_genre", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    MockPipelineStages::diversify_by_genre_stage(items_with_genres.clone(), 3)
                })
            },
        );

        // Benchmark sort by score stage
        group.bench_with_input(
            BenchmarkId::new("sort_by_score", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    MockPipelineStages::sort_by_score_stage(input_items.clone(), true)
                })
            },
        );

        // Benchmark limit stage
        group.bench_with_input(
            BenchmarkId::new("limit", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    MockPipelineStages::limit_stage(input_items.clone(), 100)
                })
            },
        );
    }

    group.finish();
}

// Benchmark complete pipeline execution
fn bench_complete_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete_pipeline");

    for item_count in [100, 500, 1000, 2000] {
        let input_items: Vec<ScoredItem> = (0..item_count)
            .map(|i| ScoredItem::new(i as i64, (i as f32) / 100.0))
            .collect();
        let context = PipelineContext::new(123);
        let watched_items = vec![1, 3, 5, 7, 9];

        group.bench_with_input(
            BenchmarkId::new("complete_pipeline", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    // Simulate complete pipeline execution
                    let mut items = input_items.clone();
                    
                    // 1. ONNX inference
                    items = MockPipelineStages::onnx_inference_stage(items, &context);
                    
                    // 2. Filter watched
                    items = MockPipelineStages::filter_watched_stage(items, &watched_items);
                    
                    // 3. Diversify by genre
                    items = MockPipelineStages::diversify_by_genre_stage(items, 3);
                    
                    // 4. Sort by score
                    items = MockPipelineStages::sort_by_score_stage(items, true);
                    
                    // 5. Limit
                    items = MockPipelineStages::limit_stage(items, 100);
                    
                    items
                })
            },
        );
    }

    group.finish();
}

// Benchmark cache operations simulation
fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    // Simulate cache operations
    for item_count in [10, 50, 100, 500] {
        let items: Vec<ScoredItem> = (0..item_count)
            .map(|i| ScoredItem::new(i as i64, (i as f32) / 100.0))
            .collect();

        // Simulate cache save
        group.bench_with_input(
            BenchmarkId::new("cache_save", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    // Simulate saving to cache (serialize to JSON)
                    let _cache_key = format!("scenario_{}_user_{}", "test", 123);
                    let _cache_value = serde_json::to_string(&items).unwrap();
                })
            },
        );

        // Simulate cache get
        group.bench_with_input(
            BenchmarkId::new("cache_get", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    // Simulate getting from cache (deserialize from JSON)
                    let cache_value = serde_json::to_string(&items).unwrap();
                    let _items: Vec<ScoredItem> = serde_json::from_str(&cache_value).unwrap();
                })
            },
        );
    }

    group.finish();
}

// Benchmark ML component simulations
fn bench_ml_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("ml_components");

    // Simulate feature extraction
    group.bench_function("feature_extraction", |b| {
        b.iter(|| {
            // Simulate feature extraction for a user
            let user_features: Vec<f32> = (0..64).map(|i| i as f32 / 64.0).collect();
            let item_features: Vec<f32> = (0..32).map(|i| i as f32 / 32.0).collect();
            
            // Simulate dot product computation
            let score: f32 = user_features.iter()
                .zip(item_features.iter())
                .map(|(u, i)| u * i)
                .sum();
            
            score
        })
    });

    // Simulate Thompson Sampling
    group.bench_function("thompson_sampling", |b| {
        b.iter(|| {
            // Simulate Thompson Sampling selection
            let mut rng = rand::thread_rng();
            let alpha: Vec<f64> = vec![1.0; 10];
            let beta: Vec<f64> = vec![1.0; 10];
            
            let samples: Vec<f64> = alpha.iter()
                .zip(beta.iter())
                .map(|(a, b)| {
                    let dist = rand_distr::Beta::new(*a, *b).unwrap();
                    dist.sample(&mut rng)
                })
                .collect();
            
            samples.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap().0
        })
    });

    // Simulate LinUCB
    group.bench_function("linucb", |b| {
        b.iter(|| {
            // Simulate LinUCB selection
            let num_arms = 5;
            let feature_dim = 10;
            
            // Initialize matrices (simplified)
            let mut A: Vec<ndarray::Array2<f64>> = vec![ndarray::Array2::eye(feature_dim); num_arms];
            let mut b: Vec<ndarray::Array1<f64>> = vec![ndarray::Array1::zeros(feature_dim); num_arms];
            
            // Simulate features
            let features = ndarray::Array1::from_vec(vec![1.0; feature_dim]);
            
            // Compute UCB scores
            let mut scores = Vec::new();
            for i in 0..num_arms {
                let theta = A[i].inv().unwrap().dot(&b[i]);
                let ucb_score = theta.dot(&features) + 0.1 * (features.dot(&A[i].inv().unwrap()).dot(&features)).sqrt();
                scores.push(ucb_score);
            }
            
            scores.iter().enumerate().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).unwrap().0
        })
    });

    group.finish();
}

// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Benchmark concurrent pipeline executions
    for concurrency in [1, 5, 10, 20] {
        let input_items: Vec<ScoredItem> = (0..100)
            .map(|i| ScoredItem::new(i as i64, (i as f32) / 100.0))
            .collect();
        let context = PipelineContext::new(123);
        let watched_items = vec![1, 3, 5, 7, 9];

        group.bench_with_input(
            BenchmarkId::new("concurrent_pipeline", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    // Simulate concurrent pipeline executions
                    let handles: Vec<_> = (0..concurrency).map(|i| {
                        std::thread::spawn(move || {
                            let mut items = input_items.clone();
                            
                            // Simulate pipeline execution
                            items = MockPipelineStages::onnx_inference_stage(items, &context);
                            items = MockPipelineStages::filter_watched_stage(items, &watched_items);
                            items = MockPipelineStages::sort_by_score_stage(items, true);
                            items = MockPipelineStages::limit_stage(items, 50);
                            
                            items.len()
                        })
                    }).collect();

                    // Wait for all threads
                    for handle in handles {
                        handle.join().unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

// Benchmark memory usage patterns
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Benchmark memory usage with large item lists
    for item_count in [1000, 5000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("large_item_list", item_count),
            &item_count,
            |b, &_count| {
                b.iter(|| {
                    // Simulate processing large item lists
                    let items: Vec<ScoredItem> = (0..item_count)
                        .map(|i| ScoredItem::new(i as i64, (i as f32) / 100.0))
                        .collect();
                    
                    // Simulate sorting and filtering
                    let mut processed = items;
                    processed.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
                    processed.truncate(1000);
                    
                    processed.len()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = bongas_benches;
    config = Criterion::default()
        .sample_size(20)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5))
        .with_plots();
    targets = 
        bench_pipeline_stages,
        bench_complete_pipeline,
        bench_cache_operations,
        bench_ml_components,
        bench_concurrent_operations,
        bench_memory_usage
);

criterion_main!(bongas_benches);