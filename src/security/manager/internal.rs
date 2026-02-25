use std::collections::HashMap;
use tokio::sync::OwnedSemaphorePermit;
use tracing::error;
use crate::error::SecurityError;
use crate::security::manager::service::SecurityManager;

impl SecurityManager {
    // ── Bulkhead ─────────────────────────────────────────────────────────

    /// Acquire bulkhead permit with timeout.
    pub(super) async fn acquire_bulkhead_permit(&self) -> Result<OwnedSemaphorePermit, SecurityError> {
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.bulkhead.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(_closed)) => Err(SecurityError::ValidationOverloaded {
                queue_depth: 0,
            }),
            Err(_timeout) => {
                error!("Bulkhead timeout: too many concurrent validation requests");
                Err(SecurityError::ValidationOverloaded {
                    queue_depth: self.bulkhead.available_permits(),
                })
            }
        }
    }

    // ── Hardware ID Caching ──────────────────────────────────────────────

    /// Get cached hardware ID or generate new one.
    pub(super) async fn get_or_generate_hardware_id(&self) -> Result<String, SecurityError> {
        {
            let cached = self.cached_hardware_id.read().await;
            if let Some(id) = &*cached {
                return Ok(id.clone());
            }
        }

        let hardware_id = self.hardware_fingerprinter.generate()?;

        {
            let mut cached = self.cached_hardware_id.write().await;
            *cached = Some(hardware_id.clone());
        }

        Ok(hardware_id)
    }

    // ── Public Accessors ─────────────────────────────────────────────────

    /// Check if license validation has been completed.
    pub async fn is_validated(&self) -> bool {
        *self.validated.read().await
    }

    /// Get cached hardware ID (available after validate_license() has run).
    pub async fn get_hardware_id(&self) -> Option<String> {
        self.cached_hardware_id.read().await.clone()
    }

    /// Get current circuit breaker states for monitoring.
    pub fn get_circuit_breaker_states(&self) -> HashMap<String, String> {
        let mut states = HashMap::new();
        states.insert(
            "license_server".to_string(),
            format!("{:?}", self.license_server_cb.current_state()),
        );
        states.insert(
            "revocation".to_string(),
            format!("{:?}", self.revocation_cb.current_state()),
        );
        states.insert(
            "heartbeat".to_string(),
            format!("{:?}", self.heartbeat_cb.current_state()),
        );
        states
    }

    /// Force circuit breaker state reset (for testing/admin).
    pub fn reset_circuit_breakers(&self) {
        self.license_server_cb.reset();
        self.revocation_cb.reset();
        self.heartbeat_cb.reset();
        tracing::info!("Security circuit breakers reset");
    }
}
