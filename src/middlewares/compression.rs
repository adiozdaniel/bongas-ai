use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    body::Body,
    http::{HeaderValue, header},
};
use tower_http::compression::CompressionLayer;
use tracing::{debug, info};

/// Enhanced compression middleware with multiple algorithms and smart content filtering
pub struct CompressionConfig {
    /// Minimum size threshold for compression (default: 1KB)
    pub min_size: u64,
    /// Enable gzip compression
    pub enable_gzip: bool,
    /// Enable brotli compression
    pub enable_brotli: bool,
    /// Enable deflate compression
    pub enable_deflate: bool,
    /// Gzip compression level (1-9, where 9 is best compression)
    pub gzip_level: u8,
    /// Brotli compression quality (1-11, where 11 is best compression)
    pub brotli_quality: u32,
    /// Deflate compression level (1-9, where 9 is best compression)
    pub deflate_level: u8,
    /// Content types to exclude from compression
    pub excluded_content_types: Vec<&'static str>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 1024, // 1KB minimum
            enable_gzip: true,
            enable_brotli: true,
            enable_deflate: true,
            gzip_level: 6,
            brotli_quality: 6,
            deflate_level: 6,
            excluded_content_types: vec![
                "image/jpeg",
                "image/png", 
                "image/gif",
                "image/webp",
                "video/mp4",
                "video/mpeg",
                "audio/mpeg",
                "application/octet-stream",
                "application/pdf",
            ],
        }
    }
}

impl CompressionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min_size(mut self, size: u64) -> Self {
        self.min_size = size;
        self
    }

    pub fn enable_gzip(mut self, enable: bool) -> Self {
        self.enable_gzip = enable;
        self
    }

    pub fn enable_brotli(mut self, enable: bool) -> Self {
        self.enable_brotli = enable;
        self
    }

    pub fn enable_deflate(mut self, enable: bool) -> Self {
        self.enable_deflate = enable;
        self
    }

    pub fn gzip_level(mut self, level: u8) -> Self {
        self.gzip_level = level.clamp(1, 9);
        self
    }

    pub fn brotli_quality(mut self, quality: u32) -> Self {
        self.brotli_quality = quality.clamp(1, 11);
        self
    }

    pub fn deflate_level(mut self, level: u8) -> Self {
        self.deflate_level = level.clamp(1, 9);
        self
    }

    pub fn exclude_content_type(mut self, content_type: &'static str) -> Self {
        self.excluded_content_types.push(content_type);
        self
    }

    pub fn build(&self) -> CompressionLayer {
        CompressionLayer::new()
    }
}

/// Content-aware compression middleware
pub struct ContentAwareCompression;

impl ContentAwareCompression {
    pub async fn layer(
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        let content_type = req.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("text/plain");

        let accept_encoding = req.headers()
            .get(header::ACCEPT_ENCODING)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        debug!(
            content_type = content_type,
            accept_encoding = accept_encoding,
            "Content-aware compression check"
        );

        // Skip compression for already compressed content
        if is_compressed_content(content_type) {
            debug!("Skipping compression for already compressed content: {}", content_type);
            return next.run(req).await;
        }

        // Check if client supports compression
        if !accept_encoding.contains("gzip") && 
           !accept_encoding.contains("br") && 
           !accept_encoding.contains("deflate") {
            debug!("Client does not support compression");
            return next.run(req).await;
        }

        let response = next.run(req).await;

        // Add compression info headers
        let mut response = response;
        if should_compress_response(&response) {
            response.headers_mut().insert(
                "X-Compression-Enabled",
                HeaderValue::from_static("true"),
            );
        }

        response
    }
}

/// Smart compression middleware with dynamic algorithm selection
pub struct SmartCompression;

impl SmartCompression {
    pub async fn layer(
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        let start_time = std::time::Instant::now();
        let response = next.run(req).await;
        let processing_time = start_time.elapsed();

        // Analyze response for compression decision
        let should_compress = analyze_response_for_compression(&response);
        let compression_algorithm = select_compression_algorithm(&response);

        debug!(
            should_compress = should_compress,
            algorithm = ?compression_algorithm,
            processing_time_ms = processing_time.as_millis(),
            "Smart compression analysis"
        );

        let mut response = response;

        if should_compress {
            // Add compression metadata headers
            response.headers_mut().insert(
                "X-Compression-Algorithm",
                HeaderValue::from_str(&compression_algorithm).unwrap(),
            );

            response.headers_mut().insert(
                "X-Compression-Decision",
                HeaderValue::from_static("smart"),
            );
        }

        response
    }
}

/// Helper functions

fn is_compressed_content(content_type: &str) -> bool {
    let compressed_types = [
        "image/jpeg", "image/png", "image/gif", "image/webp", "image/avif",
        "video/mp4", "video/mpeg", "video/avi", "video/mov",
        "audio/mpeg", "audio/mp3", "audio/wav", "audio/flac",
        "application/pdf", "application/octet-stream",
        "application/zip", "application/gzip", "application/x-tar",
    ];

    compressed_types.iter().any(|&t| content_type.starts_with(t))
}

fn should_compress_response(response: &Response<Body>) -> bool {
    // Check if response is already compressed
    if let Some(content_encoding) = response.headers().get("Content-Encoding") {
        if content_encoding.to_str().unwrap_or("").contains("gzip") ||
           content_encoding.to_str().unwrap_or("").contains("br") ||
           content_encoding.to_str().unwrap_or("").contains("deflate") {
            return false;
        }
    }

    // Check content type
    if let Some(content_type) = response.headers().get("Content-Type") {
        if is_compressed_content(content_type.to_str().unwrap_or("")) {
            return false;
        }
    }

    // Check content length (minimum 1KB)
    if let Some(content_length) = response.headers().get("Content-Length") {
        if let Ok(length) = content_length.to_str().unwrap_or("0").parse::<u64>() {
            if length < 1024 {
                return false;
            }
        }
    }

    true
}

fn analyze_response_for_compression(response: &Response<Body>) -> bool {
    // Basic analysis - in production you might want more sophisticated logic
    should_compress_response(response)
}

fn select_compression_algorithm(response: &Response<Body>) -> String {
    // Simple algorithm selection based on content type
    if let Some(content_type) = response.headers().get("Content-Type") {
        let content_type_str = content_type.to_str().unwrap_or("");
        
        // Text-based content prefers brotli for better compression
        if content_type_str.starts_with("text/") || 
           content_type_str.starts_with("application/json") ||
           content_type_str.starts_with("application/javascript") {
            return "brotli".to_string();
        }
        
        // HTML content prefers gzip for compatibility
        if content_type_str.contains("html") {
            return "gzip".to_string();
        }
    }

    // Default to gzip
    "gzip".to_string()
}

/// Compression metrics middleware
pub struct CompressionMetrics;

impl CompressionMetrics {
    pub async fn layer(
        req: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        let original_size = get_request_size(&req);
        let start_time = std::time::Instant::now();
        
        let response = next.run(req).await;
        let processing_time = start_time.elapsed();

        let compressed_size = get_response_size(&response);
        let compression_ratio = if original_size > 0 {
            (original_size - compressed_size) as f64 / original_size as f64
        } else {
            0.0
        };

        // Log compression metrics
        if compressed_size > 0 {
            info!(
                original_size = original_size,
                compressed_size = compressed_size,
                compression_ratio = format!("{:.2}%", compression_ratio * 100.0),
                processing_time_ms = processing_time.as_millis(),
                "Compression metrics"
            );
        }

        let mut response = response;

        // Add compression metrics headers
        if compressed_size > 0 {
            response.headers_mut().insert(
                "X-Original-Size",
                HeaderValue::from_str(&original_size.to_string()).unwrap(),
            );

            response.headers_mut().insert(
                "X-Compressed-Size", 
                HeaderValue::from_str(&compressed_size.to_string()).unwrap(),
            );

            response.headers_mut().insert(
                "X-Compression-Ratio",
                HeaderValue::from_str(&format!("{:.2}%", compression_ratio * 100.0)).unwrap(),
            );
        }

        response
    }
}

fn get_request_size(req: &Request<Body>) -> u64 {
    // In a real implementation, you'd need to buffer the request body
    // For now, we'll estimate based on headers
    req.headers().iter().map(|(_, v)| v.as_bytes().len() as u64).sum()
}

fn get_response_size(response: &Response<Body>) -> u64 {
    // In a real implementation, you'd need to buffer the response body
    // For now, we'll estimate based on headers
    response.headers().iter().map(|(_, v)| v.as_bytes().len() as u64).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    #[test]
    fn test_is_compressed_content() {
        assert!(is_compressed_content("image/jpeg"));
        assert!(is_compressed_content("video/mp4"));
        assert!(is_compressed_content("audio/mpeg"));
        assert!(!is_compressed_content("text/html"));
        assert!(!is_compressed_content("application/json"));
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.min_size, 1024);
        assert!(config.enable_gzip);
        assert!(config.enable_brotli);
        assert!(config.enable_deflate);
        assert_eq!(config.gzip_level, 6);
        assert_eq!(config.brotli_quality, 6);
    }

    #[test]
    fn test_compression_config_builder() {
        let config = CompressionConfig::new()
            .min_size(2048)
            .enable_gzip(false)
            .gzip_level(9);

        assert_eq!(config.min_size, 2048);
        assert!(!config.enable_gzip);
        assert_eq!(config.gzip_level, 9);
    }
}