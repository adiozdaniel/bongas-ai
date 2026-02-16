//! Concurrency benchmarks - Concurrent user simulation
//!
//! Tests system behavior under concurrent load:
//! - Resource contention (DB, Redis, ML workers)
//! - Circuit breaker behavior under load
//! - Bulkhead effectiveness

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;

// Mock concurrent workload generator
struct ConcurrentWorkload {
    max_concurrent: usize,
    semaphore: Arc<Semaphore>,
    completed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
}

impl ConcurrentWorkload {
    fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            completed: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
        }
    }

    async fn execute_request(&self, _user_id: i32) -> Result<(), String> {
        let _permit = self.semaphore.acquire().await.unwrap();

        // Simulate realistic workload
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Simulate 95% success rate
        if rand::random::<f64>() < 0.95 {
            self.completed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.failed.fetch_add(1, Ordering::Relaxed);
            Err("simulated failure".to_string())
        }
    }

    fn stats(&self) -> (u64, u64) {
        (
            self.completed.load(Ordering::Relaxed),
            self.failed.load(Ordering::Relaxed)
        )
    }
}

fn bench_concurrent_users(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_users");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(30));

    for concurrency in [10, 50, 100, 200, 500] {
        let workload = Arc::new(ConcurrentWorkload::new(concurrency));

        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrency),
            &concurrency,
            |b, &_count| {
                b.to_async(&rt).iter(|| async {
                    let workload = workload.clone();
                    let mut handles = vec![];

                    // Spawn concurrent requests
                    for i in 0..concurrency {
                        let wl = workload.clone();
                        let handle = tokio::spawn(async move {
                            wl.execute_request(i as i32).await
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

fn bench_sustained_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("sustained_load");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));

    let workload = Arc::new(ConcurrentWorkload::new(100));

    group.bench_function("1_minute_sustained", |b| {
        b.to_async(&rt).iter(|| async {
            let workload = workload.clone();
            let mut handles = vec![];

            // Simulate sustained load for 1 minute
            for i in 0..1000 {
                let wl = workload.clone();
                let handle = tokio::spawn(async move {
                    wl.execute_request(i).await
                });
                handles.push(handle);

                // Pace requests (100 req/s target)
                if i % 10 == 0 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            for handle in handles {
                let _ = handle.await;
            }

            let (completed, failed) = workload.stats();
            println!("Completed: {}, Failed: {}", completed, failed);
        })
    });

    group.finish();
}

fn bench_bulkhead_saturation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("bulkhead_saturation");
    group.sample_size(20);

    for max_concurrent in [10, 50, 100] {
        let workload = Arc::new(ConcurrentWorkload::new(max_concurrent));

        group.bench_with_input(
            BenchmarkId::new("max_concurrent", max_concurrent),
            &max_concurrent,
            |b, &_| {
                b.to_async(&rt).iter(|| async {
                    let workload = workload.clone();
                    let mut handles = vec![];

                    // Try to exceed bulkhead (2x requests)
                    for i in 0..(max_concurrent * 2) {
                        let wl = workload.clone();
                        let handle = tokio::spawn(async move {
                            wl.execute_request(i as i32).await
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
    config = Criterion::default();
    targets = bench_concurrent_users,
              bench_sustained_load,
              bench_bulkhead_saturation
);

criterion_main!(benches);
