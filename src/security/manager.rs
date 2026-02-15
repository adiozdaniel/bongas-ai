//! Netflix-grade SecurityManager with circuit breaker, bulkhead, and analytics.
//!
//! Orchestrates all 8 security layers with resilience patterns:
//! - Circuit breaker per security concern (license server, revocation, heartbeat)
//! - Bulkhead semaphore for concurrent validation requests
//! - Analytics for per-layer latency, throughput, and error tracking
//! - Fallback to degraded mode when server unavailable

use std::sync::Arc;
use std::time::Instant;
use std::collections::HashMap;
use tokio::sync::{RwLock, Semaphore, OwnedSemaphorePermit};
use tracing::{info, warn, error};

use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitBreakerError,
};
use crate::circuit_breaker::observer::{ResilienceObserver, CircuitBreakerId};
use crate::error::SecurityError;
use crate::security::{
    license::LicenseValidator,
    hardware::HardwareFingerprinter,
    anti_debug::AntiDebugDetector,
    binary::BinaryIntegrityChecker,
};
use crate::config::SecurityConfig;
use crate::analytics::types::PerformanceStats;

/// Netflix-grade SecurityManager with comprehensive resilience patterns.
///
/// Orchestrates all 8 security layers with:
/// - Per-concern circuit breakers (license_server, revocation, heartbeat)
/// - Bulkhead semaphore for concurrent validation requests
/// - Analytics for per-layer latency, throughput, and error tracking
/// - Fallback to degraded mode when server unavailable
pub struct SecurityManager {
    config: SecurityConfig,
    license_validator: Arc<LicenseValidator>,
    hardware_fingerprinter: Arc<HardwareFingerprinter>,
    anti_debug: Arc<AntiDebugDetector>,
    integrity_checker: Arc<BinaryIntegrityChecker>,

    // Per-concern circuit breakers (stored locally for direct access)
    license_server_cb: Arc<CircuitBreaker>,
    revocation_cb: Arc<CircuitBreaker>,
    heartbeat_cb: Arc<CircuitBreaker>,

    // Analytics
    analytics: Option<Arc<PerformanceStats>>,

    // Bulkhead semaphore for concurrent validation requests
    bulkhead: Arc<Semaphore>,

    // State tracking
    validated: Arc<RwLock<bool>>,
    cached_hardware_id: Arc<RwLock<Option<String>>>,
}

impl SecurityManager {
    /// Create a new SecurityManager with Netflix-grade resilience.
    pub fn new(
        config: SecurityConfig,
        circuit_breakers: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
    ) -> Result<Self, SecurityError> {
        // Build per-concern circuit breakers with config-driven settings
        let license_server_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerId::new("security.license_server"),
            CircuitBreakerConfig::builder()
                .failure_rate_threshold(config.license_server_failure_rate)
                .slow_call_rate_threshold(config.license_server_slow_call_rate)
                .recovery_timeout(config.license_server_recovery_timeout)
                .call_timeout(config.server_validation_timeout)
                .build()
                .unwrap_or_default(),
            observer.clone(),
        ));

        let revocation_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerId::new("security.revocation"),
            CircuitBreakerConfig::builder()
                .failure_rate_threshold(config.revocation_check_failure_rate)
                .slow_call_rate_threshold(config.revocation_check_slow_call_rate)
                .recovery_timeout(config.revocation_check_recovery_timeout)
                .call_timeout(config.revocation_check_timeout)
                .build()
                .unwrap_or_default(),
            observer.clone(),
        ));

        let heartbeat_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerId::new("security.heartbeat"),
            CircuitBreakerConfig::builder()
                .failure_rate_threshold(config.heartbeat_failure_rate)
                .slow_call_rate_threshold(config.heartbeat_slow_call_rate)
                .recovery_timeout(config.heartbeat_recovery_timeout)
                .call_timeout(config.server_validation_timeout)
                .build()
                .unwrap_or_default(),
            observer,
        ));

        // Register in central registry for monitoring/introspection
        circuit_breakers.register(license_server_cb.clone());
        circuit_breakers.register(revocation_cb.clone());
        circuit_breakers.register(heartbeat_cb.clone());

        // Initialize sub-modules with config
        let license_validator = Arc::new(LicenseValidator::new(&config));
        let hardware_fingerprinter = Arc::new(HardwareFingerprinter::new(&config.hardware_id_salt));
        let anti_debug = Arc::new(AntiDebugDetector::new());
        let integrity_checker = Arc::new(BinaryIntegrityChecker::new()?);

        let bulkhead = Arc::new(Semaphore::new(config.max_concurrent_validations));

        Ok(Self {
            config,
            license_validator,
            hardware_fingerprinter,
            anti_debug,
            integrity_checker,
            license_server_cb,
            revocation_cb,
            heartbeat_cb,
            analytics,
            bulkhead,
            validated: Arc::new(RwLock::new(false)),
            cached_hardware_id: Arc::new(RwLock::new(None)),
        })
    }

    /// Execute all 8 security layers with resilience patterns.
    pub async fn validate_license(&self) -> Result<(), SecurityError> {
        info!("Starting 8-layer security validation with resilience patterns...");

        // Acquire bulkhead permit for concurrent validation
        let _permit = self.acquire_bulkhead_permit().await?;

        // Layer 1: License Key Validation (local, no resilience needed)
        info!("Layer 1/8: Validating license key...");
        self.license_validator
            .validate_license_key(&self.config.license_key)?;
        info!("Layer 1 passed");

        // Layer 2: Hardware Fingerprinting (local, cache result)
        info!("Layer 2/8: Generating hardware fingerprint...");
        let hardware_id = self.get_or_generate_hardware_id().await?;
        self.license_validator
            .validate_hardware_binding(&hardware_id)?;
        info!("Layer 2 passed");

        // Layer 3: Binary Integrity Check (local)
        if self.config.binary_protection_enabled {
            info!("Layer 3/8: Verifying binary integrity...");
            self.integrity_checker.verify_self()?;
            info!("Layer 3 passed");
        } else {
            info!("Layer 3 skipped (disabled)");
        }

        // Layer 4: Anti-Debugging Detection (local)
        if self.config.anti_debug_enabled {
            info!("Layer 4/8: Checking for debuggers...");
            if self.anti_debug.is_debugger_attached()? {
                error!("Debugger detected! Terminating...");
                return Err(SecurityError::DebuggerDetected);
            }
            info!("Layer 4 passed");
        } else {
            info!("Layer 4 skipped (disabled)");
        }

        // Layer 5: Analysis Tool Detection (local)
        if self.config.anti_debug_enabled {
            info!("Layer 5/8: Checking for analysis tools...");
            if self.anti_debug.detect_analysis_tools()? {
                error!("Analysis tool detected! Terminating...");
                return Err(SecurityError::AnalysisToolDetected(
                    "Analysis tool detected".into(),
                ));
            }
            info!("Layer 5 passed");
        } else {
            info!("Layer 5 skipped (disabled)");
        }

        // Layer 6: License Server Validation (with circuit breaker)
        info!("Layer 6/8: Validating with license server...");
        self.validate_with_server_resilient(&hardware_id).await?;
        info!("Layer 6 passed");

        // Layer 7: Time-Based Expiration (local)
        info!("Layer 7/8: Checking license expiration...");
        self.license_validator.check_expiration()?;
        info!("Layer 7 passed");

        // Layer 8: Network-Based Revocation Check (with circuit breaker)
        info!("Layer 8/8: Checking revocation list...");
        self.check_revocation_resilient().await?;
        info!("Layer 8 passed");

        // Mark as validated
        *self.validated.write().await = true;
        info!("All 8 security layers passed!");

        // Start heartbeat background task
        self.start_heartbeat().await;

        Ok(())
    }

    // ── Bulkhead ─────────────────────────────────────────────────────────

    /// Acquire bulkhead permit with timeout.
    async fn acquire_bulkhead_permit(&self) -> Result<OwnedSemaphorePermit, SecurityError> {
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
    async fn get_or_generate_hardware_id(&self) -> Result<String, SecurityError> {
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

    // ── Resilient Network Calls ──────────────────────────────────────────

    /// Validate with server using circuit breaker.
    async fn validate_with_server_resilient(
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
    async fn check_revocation_resilient(&self) -> Result<(), SecurityError> {
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
    async fn start_heartbeat(&self) {
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
    async fn heartbeat_check(
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
        info!("Security circuit breakers reset");
    }
}
