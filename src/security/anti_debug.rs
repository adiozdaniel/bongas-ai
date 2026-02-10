use anyhow::Result;
use std::fs;
use std::process::Command;

pub struct AntiDebugDetector;

impl AntiDebugDetector {
    pub fn new() -> Self {
        Self
    }

    /// Check if a debugger is attached (Linux)
    pub fn is_debugger_attached(&self) -> Result<bool> {
        let status = fs::read_to_string("/proc/self/status")?;

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

    /// Detect common analysis tools
    pub fn detect_analysis_tools(&self) -> Result<bool> {
        let suspicious_processes = [
            "gdb", "lldb", "strace", "ltrace", "ida", "ida64",
            "r2", "radare2", "ghidra", "x64dbg", "ollydbg",
        ];

        let output = Command::new("ps")
            .args(["-e", "-o", "comm="])
            .output()?;

        let processes = String::from_utf8_lossy(&output.stdout);

        for proc in &suspicious_processes {
            if processes.lines().any(|line| line.trim() == *proc) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
