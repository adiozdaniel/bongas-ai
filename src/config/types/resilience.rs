  pub struct ResilienceConfig {
      pub defaults: ResilienceDefaults,
      pub overrides: HashMap<String, ServiceResilienceConfig>,
  }

  pub struct ResilienceDefaults {
      pub circuit_breaker: CircuitBreakerConfig,
      pub retry: RetryConfig,
      pub timeout: Duration,
  }
