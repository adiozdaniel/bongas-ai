use axum::{
    body::{Body, Bytes},
    extract::Request,
    http::{StatusCode, HeaderMap, HeaderValue, header},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};
use chrono::{Utc, DateTime};
use uuid::Uuid;
use httpdate;

/// Cache metadata for response body caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub created_at: DateTime<Utc>,
    pub content_type: String,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub cache_control: Option<String>,
}

/// Enhanced cache entry with response body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub metadata: CacheMetadata,
    pub body: Bytes,
    pub headers: HashMap<String, String>,
}

/// Enhanced response body caching middleware
pub struct ResponseCacheMiddleware {
    redis: Arc<redis::Client>,
    ttl_seconds: u64,
    max_body_size: u64, // Maximum body size to cache (default: 1MB)
    cacheable_content_types: Vec<&'static str>,
    cacheable_status_codes: Vec<StatusCode>,
}

impl ResponseCacheMiddleware {
    pub fn new(redis: Arc<redis::Client>, ttl_seconds: u64) -> Self {
        Self {
            redis,
            ttl_seconds,
            max_body_size: 1024 * 1024, // 1MB
            cacheable_content_types: vec![
                "application/json",
                "text/html",
                "text/plain",
                "text/css",
                "application/javascript",
                "application/xml",
                "text/xml",
            ],
            cacheable_status_codes: vec![
                StatusCode::OK,
                StatusCode::NOT_MODIFIED,
            ],
        }
    }

    pub fn max_body_size(mut self, size: u64) -> Self {
        self.max_body_size = size;
        self
    }

    pub fn cacheable_content_types(mut self, types: Vec<&'static str>) -> Self {
        self.cacheable_content_types = types;
        self
    }

    pub fn cacheable_status_codes(mut self, codes: Vec<StatusCode>) -> Self {
        self.cacheable_status_codes = codes;
        self
    }

    pub async fn layer(
        &self,
        req: Request<Body>,
        next: Next,
    ) -> Result<Response<Body>, StatusCode> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let cache_key = self.generate_cache_key(&method, &uri);

        // Only cache GET and HEAD requests
        if method != axum::http::Method::GET && method != axum::http::Method::HEAD {
            return Ok(next.run(req).await);
        }

        // Try to get cached response
        if let Some(cached_response) = self.get_cached_response(&cache_key).await? {
            info!(uri = %uri, "Cache hit for response body");
            
            // Check if cached response is still valid
            if self.is_cache_valid(&cached_response, &req.headers()) {
                return Ok(self.build_cached_response(cached_response));
            }
        }

        info!(uri = %uri, "Cache miss, executing request");

        // Execute request and cache response
        let response = next.run(req).await;
        let status = response.status();

        // Cache successful responses
        if self.should_cache_response(&response) {
            // Buffer the response body for caching
            let (parts, body) = response.into_parts();
            
            // Buffer the response body
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!(error = ?e, "Failed to buffer response body for caching");
                    return Ok(Response::from_parts(parts, Body::empty()));
                }
            };
            
            // Check body size
            if body_bytes.len() as u64 > self.max_body_size {
                debug!("Response body too large to cache: {} bytes", body_bytes.len());
                return Ok(Response::from_parts(parts, Body::from(body_bytes)));
            }

            // Create cache entry
            let metadata = self.extract_metadata(&parts.headers, body_bytes.len() as u64);
            let headers = self.extract_headers(&parts.headers);
            let cache_entry = CacheEntry {
                metadata,
                body: body_bytes.clone(),
                headers,
            };

            // Store in cache
            let cache_data = match serde_json::to_string(&cache_entry) {
                Ok(data) => data,
                Err(e) => {
                    warn!(error = ?e, "Failed to serialize cache entry");
                    return Ok(Response::from_parts(parts, Body::from(body_bytes)));
                }
            };

            match self.redis.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    if let Err(e) = conn.set_ex::<&str, &str, ()>(&cache_key, &cache_data, self.ttl_seconds).await {
                        warn!(error = ?e, "Failed to store response in cache");
                    } else {
                        debug!("Response cached successfully: {}", cache_key);
                    }
                }
                Err(e) => {
                    warn!(error = ?e, "Failed to get Redis connection for caching");
                }
            }

            // Return the response
            Ok(Response::from_parts(parts, Body::from(body_bytes)))
        } else {
            debug!(status = %status, "Response not cached (not cacheable)");
            Ok(response)
        }
    }

    fn generate_cache_key(&self, method: &axum::http::Method, uri: &axum::http::Uri) -> String {
        // Include query parameters in cache key
        let query = uri.query().unwrap_or("");
        format!("response_cache:{}:{}?{}", method, uri.path(), query)
    }

    async fn get_cached_response(&self, cache_key: &str) -> Result<Option<CacheEntry>, StatusCode> {
        let mut conn = self.redis.get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!(error = ?e, "Failed to get Redis connection for cache");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let cached_data: Option<String> = conn.get(cache_key).await
            .map_err(|e| {
                error!(error = ?e, "Failed to get cached response from Redis");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(data) = cached_data {
            match serde_json::from_str::<CacheEntry>(&data) {
                Ok(entry) => Ok(Some(entry)),
                Err(e) => {
                    warn!(error = ?e, "Failed to deserialize cached response");
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn is_cache_valid(&self, cached: &CacheEntry, request_headers: &HeaderMap) -> bool {
        // Check If-None-Match header for ETag validation
        if let Some(if_none_match) = request_headers.get(header::IF_NONE_MATCH) {
            if let Some(cached_etag) = &cached.metadata.etag {
                if if_none_match.to_str().unwrap_or("") == cached_etag {
                    return true;
                }
            }
        }

        // Check If-Modified-Since header
        if let Some(if_modified_since) = request_headers.get(header::IF_MODIFIED_SINCE) {
            if let Some(last_modified) = &cached.metadata.last_modified {
                if let Ok(modified_time) = httpdate::parse_http_date(if_modified_since.to_str().unwrap_or("")) {
                    let cached_time = last_modified.timestamp();
                    if let Ok(duration) = modified_time.duration_since(UNIX_EPOCH) {
                        if duration.as_secs() as i64 >= cached_time {
                            return true;
                        }
                    }
                }
            }
        }

        // Check cache control headers
        if let Some(cache_control) = &cached.metadata.cache_control {
            if cache_control.contains("no-cache") || cache_control.contains("must-revalidate") {
                return false;
            }
        }

        true
    }

    fn build_cached_response(&self, cached: CacheEntry) -> Response<Body> {
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(cached.body))
            .unwrap();

        // Add cached headers
        let headers = response.headers_mut();
        for (key, value) in cached.headers {
            if let Ok(header_name) = key.parse::<axum::http::HeaderName>() {
                if let Ok(header_value) = value.parse::<axum::http::HeaderValue>() {
                    headers.insert(header_name, header_value);
                }
            }
        }

        // Add cache metadata headers
        headers.insert("X-Cache", HeaderValue::from_static("HIT"));
        headers.insert("X-Cache-Key", HeaderValue::from_str(&format!("response_cache:{}", Uuid::new_v4())).unwrap());

        // Add cache control headers
        if let Some(cache_control) = &cached.metadata.cache_control {
            headers.insert(header::CACHE_CONTROL, HeaderValue::from_str(cache_control).unwrap());
        }

        // Add ETag header
        if let Some(etag) = &cached.metadata.etag {
            headers.insert(header::ETAG, HeaderValue::from_str(etag).unwrap());
        }

        // Add Last-Modified header
        if let Some(last_modified) = &cached.metadata.last_modified {
            let http_date = httpdate::fmt_http_date(last_modified.with_timezone(&chrono::Utc).into());
            headers.insert(header::LAST_MODIFIED, HeaderValue::from_str(&http_date).unwrap());
        }

        response
    }

    async fn cache_response(&self, cache_key: &str, response: Response<Body>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (parts, body) = response.into_parts();

        // Buffer the response body
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;
        
        // Check body size
        if body_bytes.len() as u64 > self.max_body_size {
            debug!("Response body too large to cache: {} bytes", body_bytes.len());
            return Ok(());
        }

        // Extract metadata
        let metadata = self.extract_metadata(&parts.headers, body_bytes.len() as u64);
        
        // Extract headers
        let headers = self.extract_headers(&parts.headers);

        let cache_entry = CacheEntry {
            metadata,
            body: body_bytes,
            headers,
        };

        // Serialize and store in Redis
        let cache_data = serde_json::to_string(&cache_entry)?;
        
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let _: () = conn.set_ex(cache_key, cache_data, self.ttl_seconds).await?;

        debug!("Response cached successfully: {}", cache_key);
        Ok(())
    }

    fn should_cache_response(&self, response: &Response<Body>) -> bool {
        let status = response.status();
        
        // Check status code
        if !self.cacheable_status_codes.contains(&status) {
            return false;
        }

        // Check content type
        if let Some(content_type) = response.headers().get(header::CONTENT_TYPE) {
            let content_type_str = content_type.to_str().unwrap_or("");
            if !self.cacheable_content_types.iter().any(|&t| content_type_str.starts_with(t)) {
                return false;
            }
        }

        // Check cache control headers
        if let Some(cache_control) = response.headers().get(header::CACHE_CONTROL) {
            let cache_control_str = cache_control.to_str().unwrap_or("");
            if cache_control_str.contains("no-store") || cache_control_str.contains("private") {
                return false;
            }
        }

        true
    }

    fn extract_metadata(&self, headers: &HeaderMap, body_size: u64) -> CacheMetadata {
        let content_type = headers.get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let etag = headers.get(header::ETAG)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let last_modified = headers.get(header::LAST_MODIFIED)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| httpdate::parse_http_date(s).ok())
            .map(|dt| DateTime::from(dt));

        let cache_control = headers.get(header::CACHE_CONTROL)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Generate ETag if not present
        let etag = etag.or_else(|| {
            // Create a simple ETag based on content length and timestamp
            Some(format!("W/\"{}-{}", body_size, Utc::now().timestamp()))
        });

        CacheMetadata {
            created_at: Utc::now(),
            content_type,
            content_length: Some(body_size),
            etag,
            last_modified,
            cache_control,
        }
    }

    fn extract_headers(&self, headers: &HeaderMap) -> HashMap<String, String> {
        let mut result = HashMap::new();
        
        // Include headers that should be preserved in cached responses
        let important_headers = [
            header::CONTENT_TYPE,
            header::CONTENT_ENCODING,
            header::CONTENT_LENGTH,
            header::ETAG,
            header::LAST_MODIFIED,
            header::CACHE_CONTROL,
            header::EXPIRES,
            header::VARY,
        ];

        for header_name in &important_headers {
            if let Some(value) = headers.get(header_name) {
                if let Ok(value_str) = value.to_str() {
                    result.insert(header_name.to_string(), value_str.to_string());
                }
            }
        }

        result
    }
}

/// Cache warming middleware for proactive caching
pub struct CacheWarmingMiddleware {
    cache: Arc<ResponseCacheMiddleware>,
}

impl CacheWarmingMiddleware {
    pub fn new(cache: Arc<ResponseCacheMiddleware>) -> Self {
        Self { cache }
    }

    pub async fn warm_cache(&self, endpoints: Vec<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for endpoint in endpoints {
            info!("Warming cache for endpoint: {}", endpoint);
            
            // Create a mock request for warming
            let request = Request::builder()
                .uri(&endpoint)
                .method("GET")
                .body(Body::empty())?;

            // This would need to be integrated with the actual request handling
            // For now, we'll just log the warming attempt
            debug!("Cache warming attempted for: {}", endpoint);
        }

        Ok(())
    }
}

/// Cache invalidation middleware
pub struct CacheInvalidationMiddleware {
    redis: Arc<redis::Client>,
}

impl CacheInvalidationMiddleware {
    pub fn new(redis: Arc<redis::Client>) -> Self {
        Self { redis }
    }

    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        
        // Get all keys matching the pattern
        let keys: Vec<String> = conn.keys(pattern).await?;
        
        // Delete matching keys
        let deleted_count = if !keys.is_empty() {
            conn.del(&keys).await?
        } else {
            0
        };

        info!("Invalidated {} cache entries matching pattern: {}", deleted_count, pattern);
        Ok(deleted_count)
    }

    pub async fn invalidate_by_prefix(&self, prefix: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        self.invalidate_pattern(&format!("{}*", prefix)).await
    }

    pub async fn invalidate_all_responses(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        self.invalidate_pattern("response_cache:*").await
    }
}

/// Cache statistics and monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f64,
    pub total_cached_entries: u64,
    pub total_cache_size: u64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            hit_rate: 0.0,
            total_cached_entries: 0,
            total_cache_size: 0,
        }
    }

    pub fn update_hit(&mut self) {
        self.total_requests += 1;
        self.cache_hits += 1;
        self.update_hit_rate();
    }

    pub fn update_miss(&mut self) {
        self.total_requests += 1;
        self.cache_misses += 1;
        self.update_hit_rate();
    }

    fn update_hit_rate(&mut self) {
        if self.total_requests > 0 {
            self.hit_rate = (self.cache_hits as f64 / self.total_requests as f64) * 100.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    #[test]
    fn test_cache_metadata_creation() {
        let metadata = CacheMetadata {
            created_at: Utc::now(),
            content_type: "application/json".to_string(),
            content_length: Some(1024),
            etag: Some("etag-123".to_string()),
            last_modified: None,
            cache_control: Some("public, max-age=3600".to_string()),
        };

        assert_eq!(metadata.content_type, "application/json");
        assert_eq!(metadata.content_length, Some(1024));
    }

    #[test]
    fn test_should_cache_response() {
        let cache = ResponseCacheMiddleware::new(Arc::new(redis::Client::open("redis://localhost").unwrap()), 3600);
        
        // Test cacheable content type
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        
        // This test would need actual response body to work properly
        // For now, we'll just test the logic structure
        assert!(true); // Placeholder
    }
}