//! Netflix-grade pipeline executor orchestrator.

use std::sync::Arc;
use std::collections::HashMap;

use crate::analytics::types::PerformanceStats;
use crate::circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig as BreakerConfig,
    CircuitBreakerRegistry, CircuitBreakerId,
};
use crate::circuit_breaker::observer::ResilienceObserver;
use crate::config::PipelineConfig;
use crate::pipeline::validator::service::PipelineValidator;
use crate::pipeline::registry::service::PipelineRegistry;

pub struct PipelineExecutor {
    pub(super) registry: PipelineRegistry,
    pub(super) stage_breakers: HashMap<String, Arc<CircuitBreaker>>,
    pub(super) validator: PipelineValidator,
    pub(super) config: PipelineConfig,
    pub(super) analytics: Option<Arc<PerformanceStats>>,
}

impl PipelineExecutor {
    pub fn new(
        config: PipelineConfig,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
    ) -> Self {
        Self::with_registry(config, breaker_registry, observer, analytics, PipelineRegistry::new())
    }

    pub fn stage_count(&self) -> usize {
        self.registry.list_stages().len()
    }

    pub fn with_registry(
        config: PipelineConfig,
        breaker_registry: Arc<CircuitBreakerRegistry>,
        observer: Arc<dyn ResilienceObserver>,
        analytics: Option<Arc<PerformanceStats>>,
        registry: PipelineRegistry,
    ) -> Self {
        let validator = PipelineValidator::new(registry.clone());
        let mut stage_breakers = HashMap::new();
        
        if config.stage_breaker_enabled {
            for stage_name in registry.list_stages() {
                let breaker_config = BreakerConfig::builder()
                    .failure_rate_threshold(config.stage_breaker_failure_rate)
                    .slow_call_rate_threshold(config.stage_breaker_slow_call_rate)
                    .slow_call_duration(config.stage_breaker_slow_call_duration)
                    .minimum_calls(config.stage_breaker_minimum_calls)
                    .recovery_timeout(config.stage_breaker_recovery_timeout)
                    .half_open_max_calls(config.stage_breaker_half_open_calls)
                    .call_timeout(Self::timeout_for_stage(&stage_name, &config))
                    .build();

                if let Ok(bc) = breaker_config {
                    let breaker_id = format!("pipeline.stage.{}", stage_name);
                    let breaker = Arc::new(CircuitBreaker::new(CircuitBreakerId::new(breaker_id), bc, observer.clone()));
                    breaker_registry.register(breaker.clone());
                    stage_breakers.insert(stage_name.clone(), breaker);
                }
            }
        }

        Self {
            registry,
            stage_breakers,
            validator,
            config,
            analytics,
        }
    }
}
