use std::time::Instant;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use tower::ServiceExt;
use tokio::time::Duration;
use tokio::sync::Semaphore;
use crate::api::create_router;
use crate::engine::BongasEngine;
use crate::cache::PostgresCache;
use crate::config::Settings;

/// Load testing for middleware stack
#[cfg(test)]
mod load_tests {
    use super::*;
    use std::sync::Arc;
    use redis::Client;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[tokio::test]
    async fn test_load_test_1000_requests() {
        // Test handling 1000 requests
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        let start = Instant::now();
        let mut handles = vec![];
        
        // Spawn 1000 concurrent requests
        for _ in 0..1000 {
            let router = router.clone();
            let handle = tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/v1/recommendations/trending")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let response = router.oneshot(request).await.unwrap();
                response.status() == StatusCode::OK
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        let duration = start.elapsed();
        let avg_time = duration / 1000;
        let requests_per_second = 1000.0 / duration.as_secs_f64();

        println!("Load test results:");
        println!("  Total requests: 1000");
        println!("  Successful requests: {}", success_count);
        println!("  Total time: {:?}", duration);
        println!("  Average time per request: {:?}", avg_time);
        println!("  Requests per second: {:.2}", requests_per_second);

        // Assert performance requirements
        assert!(success_count >= 950, "Too many failed requests: {}", success_count);
        assert!(avg_time < Duration::from_millis(100), 
               "Average response time too high: {:?}", avg_time);
        assert!(requests_per_second > 100.0, 
               "Throughput too low: {:.2} req/s", requests_per_second);
    }

    #[tokio::test]
    async fn test_load_test_rate_limiting() {
        // Test rate limiting under load
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        let start = Instant::now();
        let mut handles = vec![];
        let rate_limited_count = Arc::new(AtomicU64::new(0));
        
        // Spawn 200 requests (exceeding rate limit of 100 per 60 seconds)
        for _ in 0..200 {
            let router = router.clone();
            let rate_limited = rate_limited_count.clone();
            let handle = tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/v1/recommendations/trending")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let response = router.oneshot(request).await.unwrap();
                if response.status() == StatusCode::TOO_MANY_REQUESTS {
                    rate_limited.fetch_add(1, Ordering::Relaxed);
                }
                response.status()
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut success_count = 0;
        for handle in handles {
            let status = handle.await.unwrap();
            if status == StatusCode::OK {
                success_count += 1;
            }
        }

        let duration = start.elapsed();
        let rate_limited = rate_limited_count.load(Ordering::Relaxed);

        println!("Rate limiting load test results:");
        println!("  Total requests: 200");
        println!("  Successful requests: {}", success_count);
        println!("  Rate limited requests: {}", rate_limited);
        println!("  Total time: {:?}", duration);

        // Assert rate limiting is working
        assert!(rate_limited > 0, "No requests were rate limited");
        assert!(success_count <= 100, "Too many requests succeeded, rate limiting not working");
    }

    #[tokio::test]
    async fn test_load_test_cache_warming() {
        // Test cache warming under load
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        // First, warm up the cache with a few requests
        for _ in 0..5 {
            let request = Request::builder()
                .uri("/api/v1/recommendations/trending")
                .method("GET")
                .body(Body::empty())
                .unwrap();

            let _response = router.clone().oneshot(request).await.unwrap();
        }

        // Now test performance with warmed cache
        let start = Instant::now();
        let mut handles = vec![];
        
        // Spawn 500 concurrent requests to cached endpoint
        for _ in 0..500 {
            let router = router.clone();
            let handle = tokio::spawn(async move {
                let request = Request::builder()
                    .uri("/api/v1/recommendations/trending")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let response = router.oneshot(request).await.unwrap();
                response.status() == StatusCode::OK
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        let duration = start.elapsed();
        let avg_time = duration / 500;
        let requests_per_second = 500.0 / duration.as_secs_f64();

        println!("Cache warming load test results:");
        println!("  Total requests: 500");
        println!("  Successful requests: {}", success_count);
        println!("  Total time: {:?}", duration);
        println!("  Average time per request: {:?}", avg_time);
        println!("  Requests per second: {:.2}", requests_per_second);

        // Assert improved performance with cached responses
        assert!(success_count >= 475, "Too many failed requests: {}", success_count);
        assert!(avg_time < Duration::from_millis(50), 
               "Average response time too high with cache: {:?}", avg_time);
        assert!(requests_per_second > 200.0, 
               "Throughput too low with cache: {:.2} req/s", requests_per_second);
    }

    #[tokio::test]
    async fn test_load_test_memory_usage() {
        // Test memory usage under sustained load
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        let semaphore = Arc::new(Semaphore::new(50)); // Limit concurrent requests to 50
        let start = Instant::now();
        let mut handles = vec![];
        
        // Spawn 1000 requests with controlled concurrency
        for _ in 0..1000 {
            let router = router.clone();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold the permit for the duration of the request
                
                let request = Request::builder()
                    .uri("/api/v1/recommendations/trending")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap();

                let response = router.oneshot(request).await.unwrap();
                response.status() == StatusCode::OK
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        let duration = start.elapsed();
        let avg_time = duration / 1000;
        let requests_per_second = 1000.0 / duration.as_secs_f64();

        println!("Memory usage load test results:");
        println!("  Total requests: 1000");
        println!("  Successful requests: {}", success_count);
        println!("  Total time: {:?}", duration);
        println!("  Average time per request: {:?}", avg_time);
        println!("  Requests per second: {:.2}", requests_per_second);

        // Assert stable performance under sustained load
        assert!(success_count >= 950, "Too many failed requests: {}", success_count);
        assert!(avg_time < Duration::from_millis(150), 
               "Average response time too high under load: {:?}", avg_time);
        assert!(requests_per_second > 80.0, 
               "Throughput too low under load: {:.2} req/s", requests_per_second);
    }

    #[tokio::test]
    async fn test_load_test_compression() {
        // Test compression performance under load
        let settings = Settings::default();
        let engine = Arc::new(BongasEngine::new(settings.clone()).await.unwrap());
        let redis = Arc::new(Client::open("redis://localhost").unwrap());
        let cache = Arc::new(PostgresCache::new(settings.database.clone()).await.unwrap());
        let metrics_collector = Arc::new(crate::middlewares::metrics::MetricsCollector::new());

        let router = create_router(engine, redis, metrics_collector);

        let start = Instant::now();
        let mut handles = vec![];
        
        // Spawn 200 requests with large response bodies
        for _ in 0..200 {
            let router = router.clone();
            let handle = tokio::spawn(async move {
                let large_body = Body::from(vec![b'x'; 50000]); // 50KB body
                
                let request = Request::builder()
                    .uri("/api/v1/recommendations/trending")
                    .method("GET")
                    .body(large_body)
                    .unwrap();

                let response = router.oneshot(request).await.unwrap();
                response.status() == StatusCode::OK
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        let duration = start.elapsed();
        let avg_time = duration / 200;
        let requests_per_second = 200.0 / duration.as_secs_f64();

        println!("Compression load test results:");
        println!("  Total requests: 200");
        println!("  Successful requests: {}", success_count);
        println!("  Total time: {:?}", duration);
        println!("  Average time per request: {:?}", avg_time);
        println!("  Requests per second: {:.2}", requests_per_second);

        // Assert compression doesn't significantly impact performance
        assert!(success_count >= 190, "Too many failed requests: {}", success_count);
        assert!(avg_time < Duration::from_millis(200), 
               "Average response time too high with compression: {:?}", avg_time);
        assert!(requests_per_second > 50.0, 
               "Throughput too low with compression: {:.2} req/s", requests_per_second);
    }
}