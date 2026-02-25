//! Anti-debug detection with typed SecurityError returns.
//!
//! Detects attached debuggers and known analysis tools.
//! Returns `SecurityError` on failure for proper resilience classification.

use std::fs;
use std::process::Command;

use crate::error::SecurityError;

/// Anti-debug detector for runtime tamper detection.
pub struct AntiDebugDetector;

impl AntiDebugDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check if a debugger is attached (Linux: reads /proc/self/status TracerPid).
    pub fn is_debugger_attached(&self) -> Result<bool, SecurityError> {
        let status = fs::read_to_string("/proc/self/status").map_err(|e| {
            SecurityError::HardwareFingerprintFailed {
                reason: format!("Failed to read /proc/self/status: {}", e),
            }
        })?;

        for line in status.lines() {
            if line.starts_with("TracerPid:") {
                let tracer_pid: u32 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                if tracer_pid != 0 {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Detect common analysis tools by scanning the process list.
    pub fn detect_analysis_tools(&self) -> Result<bool, SecurityError> {
        let suspicious_processes = [
            "gdb", "lldb", "strace", "ltrace", "ida", "ida64", "r2", "radare2", "ghidra",
            "x64dbg", "ollydbg",
        ];

        let output = Command::new("ps")
            .args(["-e", "-o", "comm="])
            .output()
            .map_err(|e| SecurityError::HardwareFingerprintFailed {
                reason: format!("Failed to list processes: {}", e),
            })?;

        let processes = String::from_utf8_lossy(&output.stdout);

        for proc in &suspicious_processes {
            if processes.lines().any(|line| line.trim() == *proc) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
