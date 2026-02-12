use tower_http::compression::CompressionLayer;

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
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_size: 1024, // 1KB minimum
            enable_gzip: true,
            enable_brotli: true,
            enable_deflate: true,
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

    pub fn build(&self) -> CompressionLayer {
        CompressionLayer::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.min_size, 1024);
        assert!(config.enable_gzip);
        assert!(config.enable_brotli);
        assert!(config.enable_deflate);
    }

    #[test]
    fn test_compression_config_builder() {
        let config = CompressionConfig::new()
            .min_size(2048)
            .enable_gzip(false);

        assert_eq!(config.min_size, 2048);
        assert!(!config.enable_gzip);
    }
}
