//! Security module with Netflix-grade resilience patterns.
//!
//! Provides 8-layer security validation orchestrated by `SecurityManager`:
//! 1. License key validation (local)
//! 2. Hardware fingerprinting (local, cached)
//! 3. Binary integrity check (local, config-driven)
//! 4. Anti-debug detection (local, config-driven)
//! 5. Analysis tool detection (local, config-driven)
//! 6. License server validation (network, circuit breaker)
//! 7. License expiration check (local)
//! 8. Revocation list check (network, circuit breaker)

pub mod manager;
pub mod license;
pub mod hardware;
pub mod anti_debug;
pub mod binary;
pub mod validator;

pub use manager::SecurityManager;
pub use license::LicenseValidator;
pub use hardware::HardwareFingerprinter;
pub use anti_debug::AntiDebugDetector;
pub use binary::BinaryIntegrityChecker;
pub use validator::validate_security_config;
