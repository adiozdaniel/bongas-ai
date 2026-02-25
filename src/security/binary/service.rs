//! Binary integrity checking with typed SecurityError returns.
//!
//! Computes a SHA-256 hash at startup and verifies it hasn't changed at runtime.
//! Returns `SecurityError` on failure for proper resilience classification.

use sha2::{Sha256, Digest};
use std::fs;

use crate::error::SecurityError;

/// Binary integrity checker that detects runtime modification of the executable.
pub struct BinaryIntegrityChecker {
    expected_hash: String,
}

impl BinaryIntegrityChecker {
    /// Create a new checker, computing the baseline hash of the current binary.
    pub fn new() -> Result<Self, SecurityError> {
        let binary_path = std::env::current_exe().map_err(|e| {
            SecurityError::BinaryTampered(format!("Failed to get executable path: {}", e))
        })?;

        let binary_data = fs::read(&binary_path).map_err(|e| {
            SecurityError::BinaryTampered(format!("Failed to read binary: {}", e))
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&binary_data);
        let hash = format!("{:x}", hasher.finalize());

        Ok(Self {
            expected_hash: hash,
        })
    }

    /// Verify binary hasn't been modified since startup.
    pub fn verify_self(&self) -> Result<(), SecurityError> {
        let binary_path = std::env::current_exe().map_err(|e| {
            SecurityError::BinaryTampered(format!("Failed to get executable path: {}", e))
        })?;

        let binary_data = fs::read(&binary_path).map_err(|e| {
            SecurityError::BinaryTampered(format!("Failed to read binary: {}", e))
        })?;

        let mut hasher = Sha256::new();
        hasher.update(&binary_data);
        let current_hash = format!("{:x}", hasher.finalize());

        if current_hash != self.expected_hash {
            return Err(SecurityError::BinaryTampered(
                "Binary integrity check failed: hash mismatch".into(),
            ));
        }

        Ok(())
    }
}
