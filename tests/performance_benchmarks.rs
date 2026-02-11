use std::time::Instant;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use tower::ServiceExt;
use tokio::time::Duration;
use crate::api::create_router;
use crate::engine::BongasEngine;
use crate::cache::PostgresCache;
use crate::config::Settings;

/// Performance benchmarks for middleware stack
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::sync::Arc;
    use redis::Client;

    #[tokio::test]
    async fn benchmark_middleware_overhead() {
        // Create test router with all middleware
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        // Benchmark middleware overhead
        let start = Instant::now();
        
        for _ in 0..1000 {
            let request = Request::builder()
                .uri("/health")
                .method("GET")
                .body(Body::empty())
                .unwrap();

            let _response = router.clone().oneshot(request).await.unwrap();
        }

        let duration = start.elapsed();
        let avg_time = duration / 1000;

        println!("Average middleware overhead: {:?}", avg_time);
        
        // Assert that middleware overhead is < 5ms
        assert!(avg_time < Duration::from_millis(5), 
               "Middleware overhead too high: {:?}", avg_time);
    }

    #[tokio::test]
    async fn benchmark_cache_performance() {
        // Test cache hit vs miss performance
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        // First request (cache miss)
        let start = Instant::now();
        let request = Request::builder()
            .uri("/api/v1/recommendations/trending")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let _response = router.clone().oneshot(request).await.unwrap();
        let cache_miss_time = start.elapsed();

        // Second request (cache hit)
        let start = Instant::now();
        let request = Request::builder()
            .uri("/api/v1/recommendations/trending")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let _response = router.clone().oneshot(request).await.unwrap();
        let cache_hit_time = start.elapsed();

        println!("Cache miss time: {:?}", cache_miss_time);
        println!("Cache hit time: {:?}", cache_hit_time);

        // Cache hits should be significantly faster
        assert!(cache_hit_time < cache_miss_time / 2, 
               "Cache hit should be much faster than cache miss");
    }

    #[tokio::test]
    async fn benchmark_compression_performance() {
        // Test compression overhead
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        // Test with large response body
        let large_body = Body::from(vec![b'x'; 10000]); // 10KB body

        let start = Instant::now();
        
        for _ in 0..100 {
            let request = Request::builder()
                .uri("/api/v1/recommendations/trending")
                .method("GET")
                .body(large_body.clone())
                .unwrap();

            let _response = router.clone().oneshot(request).await.unwrap();
        }

        let duration = start.elapsed();
        let avg_time = duration / 100;

        println!("Average compression overhead: {:?}", avg_time);
        
        // Compression should not add significant overhead
        assert!(avg_time < Duration::from_millis(10), 
               "Compression overhead too high: {:?}", avg_time);
    }

    #[tokio::test]
    async fn benchmark_rate_limiting_performance() {
        // Test rate limiting overhead
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        let start = Instant::now();
        
        for _ in 0..100 {
            let request = Request::builder()
                .uri("/api/v1/recommendations/trending")
                .method("GET")
                .body(Body::empty())
                .unwrap();

            let _response = router.clone().oneshot(request).await.unwrap();
        }

        let duration = start.elapsed();
        let avg_time = duration / 100;

        println!("Average rate limiting overhead: {:?}", avg_time);
        
        // Rate limiting should not add significant overhead
        assert!(avg_time < Duration::from_millis(5), 
               "Rate limiting overhead too high: {:?}", avg_time);
    }

    #[tokio::test]
    async fn benchmark_concurrent_requests() {
        // Test concurrent request handling
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        let start = Instant::now();
        
        let mut handles = vec![];
        
        // Spawn 100 concurrent requests
        for _ in 0..100 {
            let router = router.clone();
            let handle = tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/v1/recommendations/trending")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let _response = router.oneshot(request).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            handle.await.unwrap();
        }

        let duration = start.elapsed();
        let avg_time = duration / 100;

        println!("Average concurrent request time: {:?}", avg_time);
        
        // Concurrent requests should still be fast
        assert!(avg_time < Duration::from_millis(50), 
               "Concurrent request handling too slow: {:?}", avg_time);
    }
}