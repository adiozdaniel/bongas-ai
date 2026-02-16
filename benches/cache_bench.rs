use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use bongas_ai::cache::strategies::LruCache;
use bongas_ai::cache::traits::CacheStrategy;
use bongas_ai::cache::metrics::CacheMetrics;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_lru_cache(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let metrics = Arc::new(CacheMetrics::new());
    let cache = LruCache::new(1000, metrics);
    let ttl = Duration::from_secs(60);

    let mut group = c.benchmark_group("lru_cache");

    for size in [10, 100, 500] {
        let value = vec![0.5f32; size];
        let key = "test_key";

        group.bench_with_input(BenchmarkId::new("set", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                cache.set(black_box(key), black_box(&value), black_box(ttl)).await.unwrap()
            })
        });

        rt.block_on(cache.set(key, &value, ttl)).unwrap();

        group.bench_with_input(BenchmarkId::new("get", size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res: Option<Vec<f32>> = cache.get(black_box(key)).await.unwrap();
                black_box(res)
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_lru_cache);
criterion_main!(benches);
