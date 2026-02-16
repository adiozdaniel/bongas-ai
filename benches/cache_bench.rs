//! Cache performance benchmarks
//!
//! Tests cache layer performance:
//! - L1 (LRU) hit rate and latency
//! - Cache operations under load
//! - Multi-value caching
//! - Concurrent access patterns

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use bongas_ai::cache::strategies::LruCache;
use bongas_ai::cache::traits::CacheStrategy;
use bongas_ai::cache::metrics::CacheMetrics;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_lru_cache_basic(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let metrics = Arc::new(CacheMetrics::new());
    let cache = LruCache::new(1000, metrics);
    let ttl = Duration::from_secs(60);

    let mut group = c.benchmark_group("lru_cache_basic");

    for size in [10, 100, 500, 1000] {
        let value = vec![0.5f32; size];
        let key = format!("test_key_{}", size);

        group.bench_with_input(BenchmarkId::new("set", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                cache.set(black_box(&key), black_box(&value), black_box(ttl)).await.unwrap()
            })
        });

        rt.block_on(cache.set(&key, &value, ttl)).unwrap();

        group.bench_with_input(BenchmarkId::new("get_hit", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res: Option<Vec<f32>> = cache.get(black_box(&key)).await.unwrap();
                black_box(res)
            })
        });

        group.bench_with_input(BenchmarkId::new("get_miss", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res: Option<Vec<f32>> = cache.get(black_box("missing_key")).await.unwrap();
                black_box(res)
            })
        });
    }
    group.finish();
}

fn bench_concurrent_cache_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let metrics = Arc::new(CacheMetrics::new());
    let cache = Arc::new(LruCache::new(10000, metrics));
    let ttl = Duration::from_secs(60);

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..10000 {
            let key = format!("item_{}", i);
            let value = vec![i as f32; 100];
            cache.set(&key, &value, ttl).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("concurrent_cache");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(20));

    for concurrency in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_reads", concurrency),
            &concurrency,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let cache = cache.clone();
                    let mut handles = vec![];

                    for i in 0..count {
                        let c = cache.clone();
                        let handle = tokio::spawn(async move {
                            let key = format!("item_{}", i % 10000);
                            let res: Option<Vec<f32>> = c.get(&key).await.unwrap();
                            black_box(res)
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

fn bench_cache_eviction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let ttl = Duration::from_secs(60);

    let mut group = c.benchmark_group("cache_eviction");
    group.sample_size(20);

    for cache_size in [100, 1000, 10000] {
        let metrics = Arc::new(CacheMetrics::new());
        let cache = LruCache::new(cache_size, metrics);

        group.bench_with_input(
            BenchmarkId::new("fill_and_evict", cache_size),
            &cache_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    // Write 2x items to cause 50% eviction
                    for i in 0..(size * 2) {
                        let key = format!("item_{}", i);
                        let value = vec![i as f32; 100];
                        cache.set(&key, &value, ttl).await.unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let metrics = Arc::new(CacheMetrics::new());
    let cache = Arc::new(LruCache::new(1000, metrics));
    let ttl = Duration::from_secs(60);

    // Pre-populate with 500 items
    rt.block_on(async {
        for i in 0..500 {
            let key = format!("item_{}", i);
            let value = vec![i as f32; 100];
            cache.set(&key, &value, ttl).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("mixed_workload");
    group.sample_size(50);

    group.bench_function("read_write_50_50", |b| {
        b.to_async(&rt).iter(|| async {
            let cache = cache.clone();

            for i in 0..100 {
                if i % 2 == 0 {
                    // Read
                    let key = format!("item_{}", i % 500);
                    let res: Option<Vec<f32>> = cache.get(&key).await.unwrap();
                    black_box(res);
                } else {
                    // Write
                    let key = format!("new_item_{}", i);
                    let value = vec![i as f32; 100];
                    cache.set(&key, &value, ttl).await.unwrap();
                }
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lru_cache_basic,
    bench_concurrent_cache_access,
    bench_cache_eviction,
    bench_mixed_workload
);
criterion_main!(benches);
