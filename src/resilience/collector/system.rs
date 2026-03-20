//! System health collector for real-time CPU/RAM/p99 telemetry.
//! Provides the core data feed for resource-aware resilience patterns.

use sysinfo::System;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Real-time system health metrics.
#[derive(Debug, Clone, Default)]
pub struct SystemHealth {
    pub cpu_usage: f32,
    pub mem_usage: f32,
    pub disk_usage: f32,
    pub p99_latency_ms: f32,
}

/// Collector for system health metrics with internal polling state.
pub struct SystemHealthCollector {
    system: Arc<RwLock<System>>,
    current_health: Arc<RwLock<SystemHealth>>,
}

impl SystemHealthCollector {
    /// Create a new SystemHealthCollector with initial polling state.
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system: Arc::new(RwLock::new(system)),
            current_health: Arc::new(RwLock::new(SystemHealth::default())),
        }
    }
}

impl Default for SystemHealthCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemHealthCollector {
    /// Fetch the latest cached system health snapshot.
    pub async fn get_health(&self) -> SystemHealth {
        self.current_health.read().await.clone()
    }

    /// Background task to refresh system metrics periodically.
    pub async fn start_polling(self: Arc<Self>, interval: std::time::Duration) {
        info!("SystemHealthCollector: Starting high-frequency telemetry polling...");
        
        loop {
            {
                let mut system = self.system.write().await;
                system.refresh_cpu_all();
                system.refresh_memory();
                
                let cpu_usage = system.global_cpu_usage();
                let used_mem = system.used_memory() as f32;
                let total_mem = system.total_memory() as f32;
                let mem_usage = (used_mem / total_mem) * 100.0;

                let mut health = self.current_health.write().await;
                health.cpu_usage = cpu_usage;
                health.mem_usage = mem_usage;
                
                debug!(
                    cpu = %format!("{:.2}%", cpu_usage),
                    mem = %format!("{:.2}%", mem_usage),
                    "Telemetry refresh complete"
                );
            }
            
            tokio::time::sleep(interval).await;
        }
    }
}
