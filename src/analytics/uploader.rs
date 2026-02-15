//! Statistics uploader with Netflix resilience patterns.
//!
//! Uses existing circuit breaker, bulkhead, and retry patterns from the codebase
//! for robust statistics upload to central server.

use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;

use crate::analytics::types::{ClientStatsPayload, SecurityDetails};
use crate::circuit_breaker::CircuitBreaker;
use crate::error::{ErrorClassification, ErrorClassifier};
use crate::security::SecurityManager;

/// Statistics uploader with Netflix resilience patterns.
pub struct StatsUploader {
    /// HTTP client for uploads.
    client: Client,
    
    /// Central server URL.
    central_server_url: String,
    
    /// Client identifier.
    client_id: String,
    
    /// Upload interval.
    upload_interval: Duration,
    
    /// Circuit breaker for upload protection.
    circuit_breaker: Arc<CircuitBreaker>,

    /// Security manager for real license/hardware validation.
    security_manager: Arc<SecurityManager>,
}

impl StatsUploader {
    /// Create a new statistics uploader.
    pub fn new(
        client: Client,
        central_server_url: String,
        client_id: String,
        upload_interval: Duration,
        circuit_breaker: Arc<CircuitBreaker>,
        security_manager: Arc<SecurityManager>,
    ) -> Self {
        Self {
            client,
            central_server_url,
            client_id,
            upload_interval,
            circuit_breaker,
            security_manager,
        }
    }

    /// Start background upload task.
    pub async fn start_background_upload(
        self: Arc<Self>,
        collector: Arc<crate::analytics::collector::LocalStatsCollector>,
    ) {
        let uploader = Arc::clone(&self);
        let collector = Arc::clone(&collector);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(uploader.upload_interval);
            
            loop {
                interval.tick().await;
                
                // Get security details from security manager
                let security = uploader.get_security_details().await;
                
                // Collect statistics
                let payload = collector.get_stats_for_upload(uploader.client_id.clone(), security);
                
                // Upload with resilience patterns
                if let Err(e) = uploader.upload_with_resilience(payload).await {
                    tracing::error!("Failed to upload statistics: {}", e);
                }
            }
        });
    }

    /// Upload statistics with Netflix resilience patterns.
    async fn upload_with_resilience(&self, payload: ClientStatsPayload) -> Result<(), UploadError> {
        // Use circuit breaker to protect against cascading failures.
        // CircuitBreaker::call() handles state checks, success/failure recording,
        // and observer notifications internally.
        self.circuit_breaker
            .call(|| self.upload_stats(payload))
            .await
            .map_err(|cb_err| match cb_err {
                crate::circuit_breaker::CircuitBreakerError::Rejected { .. } => {
                    UploadError::CircuitOpen
                }
                crate::circuit_breaker::CircuitBreakerError::ExecutionFailed { source, .. } => {
                    source
                }
                crate::circuit_breaker::CircuitBreakerError::TimedOut { timeout } => {
                    UploadError::Timeout(timeout.as_millis() as u64)
                }
            })
    }

    /// Upload statistics to central server.
    async fn upload_stats(&self, payload: ClientStatsPayload) -> Result<(), UploadError> {
        let url = format!("{}/api/v1/stats/upload", self.central_server_url);
        
        let response = self.client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(UploadError::Network)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(UploadError::Http(response.status().as_u16()))
        }
    }

    /// Get security details from the real SecurityManager.
    async fn get_security_details(&self) -> SecurityDetails {
        let license_valid = self.security_manager.is_validated().await;
        let hardware_fingerprint = self
            .security_manager
            .get_hardware_id()
            .await
            .unwrap_or_else(|| "unknown".to_string());

        SecurityDetails::new(
            license_valid,
            hardware_fingerprint,
            env!("CARGO_PKG_VERSION").to_string(),
        )
    }
}

/// Upload error types.
#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    /// Network error during upload.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// HTTP error with status code.
    #[error("HTTP error: {0}")]
    Http(u16),

    /// Circuit breaker is open.
    #[error("Circuit breaker is open")]
    CircuitOpen,

    /// Upload timed out.
    #[error("Upload timed out after {0}ms")]
    Timeout(u64),
}

impl ErrorClassifier for UploadError {
    fn classify(&self) -> ErrorClassification {
        match self {
            UploadError::Network(_) => ErrorClassification::Transient,
            UploadError::Http(status) if *status >= 500 => ErrorClassification::Transient,
            UploadError::Http(_) => ErrorClassification::Permanent,
            UploadError::CircuitOpen => ErrorClassification::Overload,
            UploadError::Timeout(_) => ErrorClassification::Timeout,
        }
    }
}

impl StatsUploader {
    /// Get upload interval.
    pub fn upload_interval(&self) -> Duration {
        self.upload_interval
    }

    /// Get central server URL.
    pub fn central_server_url(&self) -> &str {
        &self.central_server_url
    }

    /// Get client ID.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}