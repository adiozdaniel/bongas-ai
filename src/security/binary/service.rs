//! Binary integrity checking with typed SecurityError returns.

use sha2::{Sha256, Digest};
use std::fs;
use tracing::info;

use crate::error::SecurityError;

/// Binary integrity checker that detects runtime modification of the executable.
pub struct BinaryIntegrityChecker {
    expected_hash: String,
}

impl BinaryIntegrityChecker {
    /// Create a new checker, computing the baseline hash of the current binary.
    /// Uses blocking I/O wrapped in spawn_blocking to prevent engine hang.
    pub async fn new() -> Result<Self, SecurityError> {
        info!("Computing binary integrity baseline...");
        
        let hash = tokio::task::spawn_blocking(|| -> Result<String, SecurityError> {
            let binary_path = std::env::current_exe().map_err(|e| {
                SecurityError::BinaryTampered(format!("Failed to get executable path: {}", e))
            })?;

            let binary_data = fs::read(&binary_path).map_err(|e| {
                SecurityError::BinaryTampered(format!("Failed to read binary: {}", e))
            })?;

            let mut hasher = Sha256::new();
            hasher.update(&binary_data);
            Ok(format!("{:x}", hasher.finalize()))
        })
        .await
        .map_err(|_| SecurityError::Internal("Thread panic during binary hashing".into()))??;

        Ok(Self {
            expected_hash: hash,
        })
    }

    /// Verify binary hasn't been modified since startup.
    pub async fn verify_self(&self) -> Result<(), SecurityError> {
        let expected = self.expected_hash.clone();
        
        tokio::task::spawn_blocking(move || -> Result<(), SecurityError> {
            let binary_path = std::env::current_exe().map_err(|e| {
                SecurityError::BinaryTampered(format!("Failed to get executable path: {}", e))
            })?;

            let binary_data = fs::read(&binary_path).map_err(|e| {
                SecurityError::BinaryTampered(format!("Failed to read binary: {}", e))
            })?;

            let mut hasher = Sha256::new();
            hasher.update(&binary_data);
            let current_hash = format!("{:x}", hasher.finalize());

            if current_hash != expected {
                return Err(SecurityError::BinaryTampered(
                    "Binary integrity check failed: hash mismatch".into(),
                ));
            }

            Ok(())
        })
        .await
        .map_err(|_| SecurityError::Internal("Thread panic during binary hashing".into()))?
    }
}
