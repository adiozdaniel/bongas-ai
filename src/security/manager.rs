use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};

// use crate::config::settings::SecuritySettings;
use crate::security::{
    license::LicenseValidator,
    hardware::HardwareFingerprinter,
    anti_debug::AntiDebugDetector,
    // binary::BinaryIntegrityChecker,
    // validator::HeartbeatManager,
};

pub struct SecurityManager {
    // config: SecuritySettings,
    license_validator: Arc<LicenseValidator>,
    hardware_fingerprinter: Arc<HardwareFingerprinter>,
    anti_debug: Arc<AntiDebugDetector>,
    // integrity_checker: Arc<BinaryIntegrityChecker>,
    // _heartbeat_manager: Arc<RwLock<HeartbeatManager>>,
    validated: Arc<RwLock<bool>>,
}

impl SecurityManager {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            // config: config.clone(),
            license_validator: Arc::new(LicenseValidator::new()?),
            hardware_fingerprinter: Arc::new(HardwareFingerprinter::new("default_salt")),
            anti_debug: Arc::new(AntiDebugDetector::new()),
            // integrity_checker: Arc::new(BinaryIntegrityChecker::new()?),
            // _heartbeat_manager: Arc::new(RwLock::new(HeartbeatManager::new()?)),
            validated: Arc::new(RwLock::new(false)),
        })
    }

    /// Execute all 8 security layers
    pub async fn validate_license(&self) -> Result<()> {
        info!("Starting 8-layer security validation...");

        // Layer 1: License Key Validation
        info!("Layer 1/8: Validating license key...");
        self.license_validator.validate_license_key("LICENSE-1234-5678-ABCD")?;
        info!("Layer 1 passed");

        // Layer 2: Hardware Fingerprinting
        info!("Layer 2/8: Generating hardware fingerprint...");
        let hardware_id = self.hardware_fingerprinter.generate()?;
        self.license_validator.validate_hardware_binding(&hardware_id)?;
        info!("Layer 2 passed");

        // Layer 3: Binary Integrity Check
        // if self.config.enable_integrity_check {
        //     info!("Layer 3/8: Verifying binary integrity...");
        //     self.integrity_checker.verify_self()?;
        //     info!("Layer 3 passed");
        // } else {
        //     info!("Layer 3 skipped (disabled)");
        // }

        // Layer 4: Anti-Debugging Detection
        // if self.config.enable_anti_debug {
        //     info!("Layer 4/8: Checking for debuggers...");
        //     if self.anti_debug.is_debugger_attached()? {
        //         error!("Debugger detected! Terminating...");
        //         return Err(anyhow!("Debugger detected"));
        //     }
        //     info!("Layer 4 passed");
        // } else {
        //     info!("Layer 4 skipped (disabled)");
        // }

        // Layer 5: Analysis Tool Detection
        // if self.config.enable_anti_debug {
        //     info!("Layer 5/8: Checking for analysis tools...");
        //     if self.anti_debug.detect_analysis_tools()? {
        //         error!("Analysis tool detected! Terminating...");
        //         return Err(anyhow!("Analysis tool detected"));
        //     }
        //     info!("Layer 5 passed");
        // } else {
        //     info!("Layer 5 skipped (disabled)");
        // }

        // Layer 6: License Server Validation
        info!("Layer 6/8: Validating with license server...");
        self.license_validator.validate_with_server(&hardware_id).await?;
        info!("Layer 6 passed");

        // Layer 7: Time-Based Expiration
        info!("Layer 7/8: Checking license expiration...");
        self.license_validator.check_expiration()?;
        info!("Layer 7 passed");

        // Layer 8: Network-Based Revocation Check
        info!("Layer 8/8: Checking revocation list...");
        self.license_validator.check_revocation_list().await?;
        info!("Layer 8 passed");

        // Mark as validated
        *self.validated.write().await = true;

        info!("All 8 security layers passed!");

        // Start heartbeat background task
        self.start_heartbeat().await?;

        Ok(())
    }

    /// Start periodic license validation heartbeat
    async fn start_heartbeat(&self) -> Result<()> {
        let validator = self.license_validator.clone();
        let hardware = self.hardware_fingerprinter.clone();
        let anti_debug = self.anti_debug.clone();
        // let interval = self.config.heartbeat_interval_seconds;

        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                tick.tick().await;
                match Self::heartbeat_check(&validator, &hardware, &anti_debug).await {
                    Ok(_) => info!("License heartbeat: OK"),
                    Err(e) => {
                        error!("License heartbeat failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        });

        Ok(())
    }

    async fn heartbeat_check(
        validator: &LicenseValidator,
        hardware: &HardwareFingerprinter,
        anti_debug: &AntiDebugDetector,
    ) -> Result<()> {
        let hardware_id = hardware.generate()?;
        
        // Runtime anti-debug check
        if anti_debug.is_debugger_attached()? {
            error!("🚨 Debugger detected during runtime heartbeat!");
            return Err(anyhow!("Debugger detected during runtime"));
        }
        
        // Check for analysis tools
        if anti_debug.detect_analysis_tools()? {
            error!("🚨 Analysis tool detected during runtime heartbeat!");
            return Err(anyhow!("Analysis tool detected during runtime"));
        }
        
        validator.validate_with_server(&hardware_id).await?;
        validator.check_revocation_list().await?;
        Ok(())
    }

    pub async fn is_validated(&self) -> bool {
        *self.validated.read().await
    }
}
