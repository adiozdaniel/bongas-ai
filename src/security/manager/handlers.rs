use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn, error};
use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
use crate::error::SecurityError;
use crate::security::manager::service::SecurityManager;
use crate::security::license::LicenseValidator;
use crate::security::hardware::HardwareFingerprinter;
use crate::security::anti_debug::AntiDebugDetector;

impl SecurityManager {
    // ── Resilient Network Calls ──────────────────────────────────────────

    /// Validate with server using circuit breaker.
    pub(super) async fn validate_with_server_resilient(
        &self,
        hardware_id: &str,
    ) -> Result<(), SecurityError> {
        let hardware_id = hardware_id.to_string();
        let validator = self.license_validator.clone();
        let analytics = self.analytics.clone();

        let result = self
            .license_server_cb
            .call(move || {
                let hardware_id = hardware_id.clone();
                let validator = validator.clone();
                let analytics = analytics.clone();
                async move {
                    let start = Instant::now();
                    let result = validator.validate_with_server(&hardware_id).await;
                    let duration = start.elapsed();

                    if let Some(ref a) = analytics {
                        a.record_response_time(
                            "security.license_server",
                            duration.as_millis() as u64,
                        );
                        a.increment_throughput("security.license_server");
                    }

                    result
                }
            })
            .await;

        match result {
            Ok(()) => Ok(()),
            Err(CircuitBreakerError::Rejected { .. }) => {
                if self.config.allow_degraded_mode {
                    warn!("License server circuit breaker open, using degraded mode");
                    if let Some(ref a) = self.analytics {
                        a.increment_error("security.license_server.degraded");
                    }
                    Ok(())
                } else {
                    Err(SecurityError::CircuitOpen)
                }
            }
            Err(CircuitBreakerError::TimedOut { timeout }) => {
                if self.config.fallback_on_server_timeout {
                    warn!(
                        timeout_ms = timeout.as_millis() as u64,
                        "License server timed out, using degraded mode"
                    );
                    if let Some(ref a) = self.analytics {
                        a.increment_error("security.license_server.timeout");
                    }
                    Ok(())
                } else {
                    Err(SecurityError::ServerTimeout {
                        timeout_ms: timeout.as_millis() as u64,
                    })
                }
            }
            Err(CircuitBreakerError::ExecutionFailed { source, .. }) => {
                if self.config.fallback_on_server_error {
                    warn!(error = %source, "License server failed, using degraded mode");
                    if let Some(ref a) = self.analytics {
                        a.increment_error("security.license_server.fallback");
                    }
                    Ok(())
                } else {
                    Err(source)
                }
            }
        }
    }

    /// Check revocation with circuit breaker.
    pub(super) async fn check_revocation_resilient(&self) -> Result<(), SecurityError> {
        let validator = self.license_validator.clone();
        let analytics = self.analytics.clone();

        let result = self
            .revocation_cb
            .call(move || {
                let validator = validator.clone();
                let analytics = analytics.clone();
                async move {
                    let start = Instant::now();
                    let result = validator.check_revocation_list().await;
                    let duration = start.elapsed();

                    if let Some(ref a) = analytics {
                        a.record_response_time(
                            "security.revocation",
                            duration.as_millis() as u64,
                        );
                        a.increment_throughput("security.revocation");
                    }

                    result
                }
            })
            .await;

        match result {
            Ok(()) => Ok(()),
            Err(CircuitBreakerError::Rejected { .. }) => {
                if self.config.allow_degraded_mode {
                    warn!("Revocation check circuit breaker open, using degraded mode");
                    if let Some(ref a) = self.analytics {
                        a.increment_error("security.revocation.degraded");
                    }
                    Ok(())
                } else {
                    Err(SecurityError::CircuitOpen)
                }
            }
            Err(CircuitBreakerError::TimedOut { timeout }) => {
                if self.config.fallback_on_server_timeout {
                    warn!(
                        timeout_ms = timeout.as_millis() as u64,
                        "Revocation check timed out, using degraded mode"
                    );
                    if let Some(ref a) = self.analytics {
                        a.increment_error("security.revocation.timeout");
                    }
                    Ok(())
                } else {
                    Err(SecurityError::ServerTimeout {
                        timeout_ms: timeout.as_millis() as u64,
                    })
                }
            }
            Err(CircuitBreakerError::ExecutionFailed { source, .. }) => {
                if self.config.fallback_on_server_error {
                    warn!(error = %source, "Revocation check failed, using degraded mode");
                    if let Some(ref a) = self.analytics {
                        a.increment_error("security.revocation.fallback");
                    }
                    Ok(())
                } else {
                    Err(source)
                }
            }
        }
    }

    // ── Heartbeat ────────────────────────────────────────────────────────

    /// Start periodic license validation heartbeat with resilience.
    pub(super) async fn start_heartbeat(&self) {
        let validator = self.license_validator.clone();
        let hardware = self.hardware_fingerprinter.clone();
        let anti_debug = self.anti_debug.clone();
        let heartbeat_cb = self.heartbeat_cb.clone();
        let interval = self.config.heartbeat_interval;

        tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            loop {
                tick.tick().await;
                match Self::heartbeat_check(&validator, &hardware, &anti_debug, &heartbeat_cb)
                    .await
                {
                    Ok(_) => info!("License heartbeat: OK"),
                    Err(e) => {
                        error!("License heartbeat failed: {}", e);
                    }
                }
            }
        });
    }

    /// Single heartbeat check with circuit breaker protection.
    pub(super) async fn heartbeat_check(
        validator: &Arc<LicenseValidator>,
        hardware: &Arc<HardwareFingerprinter>,
        anti_debug: &Arc<AntiDebugDetector>,
        circuit_breaker: &Arc<CircuitBreaker>,
    ) -> Result<(), SecurityError> {
        let hardware_id = hardware.generate()?;

        // Runtime anti-debug check
        if anti_debug.is_debugger_attached()? {
            error!("Debugger detected during runtime heartbeat!");
            return Err(SecurityError::DebuggerDetected);
        }

        // Check for analysis tools
        if anti_debug.detect_analysis_tools()? {
            error!("Analysis tool detected during runtime heartbeat!");
            return Err(SecurityError::AnalysisToolDetected(
                "Analysis tool detected during runtime".into(),
            ));
        }

        // Server validation via circuit breaker
        let validator = validator.clone();
        circuit_breaker
            .call(move || {
                let validator = validator.clone();
                let hardware_id = hardware_id.clone();
                async move { validator.validate_with_server(&hardware_id).await }
            })
            .await
            .map_err(|cb_err| match cb_err {
                CircuitBreakerError::Rejected { .. } => SecurityError::CircuitOpen,
                CircuitBreakerError::TimedOut { timeout } => SecurityError::HeartbeatTimeout {
                    timeout_ms: timeout.as_millis() as u64,
                },
                CircuitBreakerError::ExecutionFailed { source, .. } => source,
            })
    }
}
