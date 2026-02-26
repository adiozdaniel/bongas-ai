use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;
use crate::circuit_breaker::CircuitBreakerConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceResilienceConfig {
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResilienceConfig {
    pub defaults: ResilienceDefaults,
    pub overrides: HashMap<String, ServiceResilienceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResilienceDefaults {
    pub circuit_breaker: CircuitBreakerConfig,
    pub retry: RetryConfig,
    pub timeout: Duration,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            defaults: ResilienceDefaults {
                circuit_breaker: CircuitBreakerConfig::default(),
                retry: RetryConfig {
                    max_retries: 3,
                    base_delay: Duration::from_millis(100),
                    max_delay: Duration::from_secs(1),
                },
                timeout: Duration::from_secs(30),
            },
            overrides: HashMap::new(),
        }
    }
}
