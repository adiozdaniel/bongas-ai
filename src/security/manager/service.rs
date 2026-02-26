//! Netflix-grade SecurityManager with circuit breaker, bulkhead, and analytics.

use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{info, warn, error};

use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry,
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
pub struct SecurityManager {
    pub(super) config: SecurityConfig,
    pub(super) license_validator: Arc<LicenseValidator>,
    pub(super) hardware_fingerprinter: Arc<HardwareFingerprinter>,
    pub(super) anti_debug: Arc<AntiDebugDetector>,
    pub(super) integrity_checker: Option<Arc<BinaryIntegrityChecker>>,

    // Per-concern circuit breakers
    pub(super) license_server_cb: Arc<CircuitBreaker>,
    pub(super) revocation_cb: Arc<CircuitBreaker>,
    pub(super) heartbeat_cb: Arc<CircuitBreaker>,

    // Analytics
    pub(super) analytics: Option<Arc<PerformanceStats>>,

    // Bulkhead semaphore for concurrent validation requests
    pub(super) bulkhead: Arc<Semaphore>,

    // State tracking
    pub(super) validated: Arc<RwLock<bool>>,
    pub(super) cached_hardware_id: Arc<RwLock<Option<String>>>,
}

impl SecurityManager {
    /// Create a new SecurityManager with Netflix-grade resilience.
    pub async fn new(
        config: SecurityConfig,
        environment: &str,
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
        
        // Skip heavy hashing in development mode to prevent bootstrap hangs
        let integrity_checker = if environment.to_lowercase() == "development" || environment.to_lowercase() == "dev" {
            info!("Development environment detected: skipping binary integrity baseline (speed optimization)");
            None
        } else {
            Some(Arc::new(BinaryIntegrityChecker::new().await?))
        };

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
            if let Some(ref checker) = self.integrity_checker {
                info!("Layer 3/8: Verifying binary integrity...");
                checker.verify_self().await?;
                info!("Layer 3 passed");
            } else {
                warn!("Layer 3/8: Binary integrity check requested but checker not initialized (dev mode bypass)");
            }
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
}
