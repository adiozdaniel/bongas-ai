use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;
use crate::config::types::circuit_breaker::CircuitBreakerConfig;

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
