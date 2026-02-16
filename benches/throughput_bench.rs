//! Throughput benchmarks - Requests per second capability
//!
//! Tests system throughput under various scenarios:
//! - Cache hits (best case)
//! - Cache misses (realistic load)
//! - Mixed workload (production simulation)

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

// Mock types for benchmarking (replace with actual types when running)
#[derive(Clone)]
struct MockEngine {
    cache_hit_rate: f64,
}

impl MockEngine {
    fn new(cache_hit_rate: f64) -> Self {
        Self { cache_hit_rate }
    }

    async fn execute_scenario(&self, _scenario: &str, _user_id: i32) -> Result<Vec<i32>, String> {
        // Simulate cache hit
        if rand::random::<f64>() < self.cache_hit_rate {
            tokio::time::sleep(Duration::from_micros(500)).await; // 0.5ms cache hit
        } else {
            tokio::time::sleep(Duration::from_millis(50)).await; // 50ms cache miss
        }
        Ok((0..10).collect())
    }
}

fn bench_cache_hit_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(MockEngine::new(1.0)); // 100% cache hits

    let mut group = c.benchmark_group("throughput_cache_hits");
    group.throughput(Throughput::Elements(1));
    group.sample_size(100);

    group.bench_function("single_request", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = engine.clone();
            engine.execute_scenario("trending_now", 123).await.unwrap()
        })
    });

    group.finish();
}

fn bench_cache_miss_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(MockEngine::new(0.0)); // 0% cache hits

    let mut group = c.benchmark_group("throughput_cache_miss");
    group.throughput(Throughput::Elements(1));
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("single_request", |b| {
        b.to_async(&rt).iter(|| async {
            let engine = engine.clone();
            engine.execute_scenario("personalized_home", 123).await.unwrap()
        })
    });

    group.finish();
}

fn bench_mixed_workload_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("throughput_mixed_workload");
    group.throughput(Throughput::Elements(100));
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    for cache_hit_rate in [0.5, 0.8, 0.95] {
        let engine = Arc::new(MockEngine::new(cache_hit_rate));

        group.bench_with_input(
            BenchmarkId::new("cache_hit_rate", (cache_hit_rate * 100.0) as u32),
            &cache_hit_rate,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let engine = engine.clone();
                    let mut handles = vec![];

                    // Simulate 100 concurrent requests
                    for i in 0..100 {
                        let eng = engine.clone();
                        let handle = tokio::spawn(async move {
                            eng.execute_scenario("trending_now", 1000 + i).await
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let _ = handle.await;
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_requests_per_second(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = Arc::new(MockEngine::new(0.9)); // 90% cache hit (production typical)

    let mut group = c.benchmark_group("req_per_sec");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    for req_count in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(req_count));

        group.bench_with_input(
            BenchmarkId::new("burst_load", req_count),
            &req_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let engine = engine.clone();
                    let mut handles = vec![];

                    for i in 0..count {
                        let eng = engine.clone();
                        let handle = tokio::spawn(async move {
                            eng.execute_scenario("trending_now", i as i32).await
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let _ = handle.await;
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(20));
    targets = bench_cache_hit_throughput,
              bench_cache_miss_throughput,
              bench_mixed_workload_throughput,
              bench_requests_per_second
);

criterion_main!(benches);
