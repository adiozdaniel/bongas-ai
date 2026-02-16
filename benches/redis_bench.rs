use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use bongas_ai::cache::strategies::RedisCache;
use bongas_ai::cache::traits::CacheStrategy;
use bongas_ai::cache::metrics::CacheMetrics;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

fn bench_redis_serialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let metrics = Arc::new(CacheMetrics::new());
    
    // Assumes redis is running on localhost:6379
    let redis_url = "redis://127.0.0.1:6379";
    let cache = match rt.block_on(RedisCache::new(redis_url, metrics)) {
        Ok(cache) => cache,
        Err(_) => {
            eprintln!("Redis not available, skipping bench_redis_serialization");
            return;
        }
    };
    
    let ttl = Duration::from_secs(60);
    let mut group = c.benchmark_group("redis_binary_serialization");

    // Bench with typical recommendation payload sizes
    for item_count in [10, 50, 100, 500] {
        let items: Vec<i32> = (0..item_count).collect();
        let key = format!("bench_items_{}", item_count);

        group.bench_with_input(BenchmarkId::new("set_bincode", item_count), &item_count, |b, _| {
            b.to_async(&rt).iter(|| async {
                cache.set(black_box(&key), black_box(&items), black_box(ttl)).await.unwrap()
            })
        });

        rt.block_on(cache.set(&key, &items, ttl)).unwrap();

        group.bench_with_input(BenchmarkId::new("get_bincode", item_count), &item_count, |b, _| {
            b.to_async(&rt).iter(|| async {
                let res: Option<Vec<i32>> = cache.get(black_box(&key)).await.unwrap();
                black_box(res)
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_redis_serialization);
criterion_main!(benches);
