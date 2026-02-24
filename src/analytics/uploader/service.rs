//! Statistics uploader implementation.

use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;

use crate::analytics::types::{ClientStatsPayload, SecurityDetails};
use crate::circuit_breaker::CircuitBreaker;
use crate::error::{ErrorClassification, ErrorClassifier};
use crate::security::SecurityManager;

/// Statistics uploader with Netflix resilience patterns.
pub struct StatsUploader {
    pub(crate) client: Client,
    pub(crate) central_server_url: String,
    pub(crate) client_id: String,
    pub(crate) upload_interval: Duration,
    pub(crate) circuit_breaker: Arc<CircuitBreaker>,
    pub(crate) security_manager: Arc<SecurityManager>,
}

impl StatsUploader {
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
                let security = uploader.get_security_details().await;
                let payload = collector.get_stats_for_upload(uploader.client_id.clone(), security);
                
                if let Err(e) = uploader.upload_with_resilience(payload).await {
                    tracing::error!("Failed to upload statistics: {}", e);
                }
            }
        });
    }

    async fn upload_with_resilience(&self, payload: ClientStatsPayload) -> Result<(), UploadError> {
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

    pub fn upload_interval(&self) -> Duration {
        self.upload_interval
    }

    pub fn central_server_url(&self) -> &str {
        &self.central_server_url
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("HTTP error: {0}")]
    Http(u16),

    #[error("Circuit breaker is open")]
    CircuitOpen,

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
