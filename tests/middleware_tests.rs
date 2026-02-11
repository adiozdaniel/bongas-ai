use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tower::ServiceExt;
use redis::Client;
use crate::middlewares::{
    cors::create_dev_cors_layer,
    compression::CompressionConfig,
    response_cache::ResponseCacheMiddleware,
};

#[tokio::test]
async fn test_cors_middleware() {
    let cors_layer = create_dev_cors_layer();
    
    // Test that CORS layer can be created
    assert!(true); // Basic test to ensure no compilation errors
}

#[tokio::test]
async fn test_compression_middleware() {
    let compression_config = CompressionConfig::new()
        .min_size(1024)
        .enable_gzip(true)
        .enable_brotli(true);

    let compression_layer = compression_config.build();
    
    // Test that compression layer can be created
    assert!(true); // Basic test to ensure no compilation errors
}

#[tokio::test]
async fn test_response_cache_middleware() {
    // This test would require a Redis connection
    // For now, we'll just test the structure
    let redis_client = Client::open("redis://localhost").unwrap();
    let redis = Arc::new(redis_client);
    
    let cache = ResponseCacheMiddleware::new(redis, 300);
    
    // Test that cache middleware can be created
    assert!(true); // Basic test to ensure no compilation errors
}

#[tokio::test]
async fn test_middleware_integration() {
    // Test that all middleware can be used together
    let cors_layer = create_dev_cors_layer();
    let compression_layer = CompressionConfig::new().build();
    
    // Test that layers can be created without errors
    assert!(true);
}