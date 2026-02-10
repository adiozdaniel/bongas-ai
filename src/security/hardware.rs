use anyhow::Result;
use sha2::{Sha256, Digest};
use std::process::Command;

pub struct HardwareFingerprinter {
    salt: String,
}

impl HardwareFingerprinter {
    pub fn new(salt: &str) -> Self {
        Self {
            salt: salt.to_string(),
        }
    }

    /// Generate hardware fingerprint from CPU ID, MAC address, and disk serial
    pub fn generate(&self) -> Result<String> {
        let mut components = Vec::new();

        if let Ok(cpu_id) = self.get_cpu_id() {
            components.push(cpu_id);
        }

        if let Ok(mac) = self.get_mac_address() {
            components.push(mac);
        }

        if let Ok(disk_serial) = self.get_disk_serial() {
            components.push(disk_serial);
        }

        let combined = format!(
            "{}{}{}",
            components.join("|"),
            self.salt,
            env!("CARGO_PKG_VERSION")
        );
        let mut hasher = Sha256::new();
        hasher.update(combined.as_bytes());
        let fingerprint = format!("{:x}", hasher.finalize());

        Ok(fingerprint)
    }

    fn get_cpu_id(&self) -> Result<String> {
        let output = Command::new("cat")
            .arg("/proc/cpuinfo")
            .output()?;

        let content = String::from_utf8_lossy(&output.stdout);
        for line in content.lines() {
            if line.starts_with("processor") {
                return Ok(line.to_string());
            }
        }

        Ok("unknown".to_string())
    }

    fn get_mac_address(&self) -> Result<String> {
        let output = Command::new("cat")
            .arg("/sys/class/net/eth0/address")
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_disk_serial(&self) -> Result<String> {
        let output = Command::new("lsblk")
            .args(["-o", "SERIAL", "-n"])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string())
    }
}
