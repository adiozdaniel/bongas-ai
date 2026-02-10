use anyhow::Result;
use sha2::{Sha256, Digest};
use std::fs;

pub struct BinaryIntegrityChecker {
    expected_hash: String,
}

impl BinaryIntegrityChecker {
    pub fn new() -> Result<Self> {
        let binary_path = std::env::current_exe()?;
        let binary_data = fs::read(&binary_path)?;

        let mut hasher = Sha256::new();
        hasher.update(&binary_data);
        let hash = format!("{:x}", hasher.finalize());

        Ok(Self {
            expected_hash: hash,
        })
    }

    /// Verify binary hasn't been modified since startup
    pub fn verify_self(&self) -> Result<()> {
        let binary_path = std::env::current_exe()?;
        let binary_data = fs::read(&binary_path)?;

        let mut hasher = Sha256::new();
        hasher.update(&binary_data);
        let current_hash = format!("{:x}", hasher.finalize());

        if current_hash != self.expected_hash {
            return Err(anyhow::anyhow!("Binary integrity check failed"));
        }

        Ok(())
    }
}
