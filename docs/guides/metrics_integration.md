# Analytics Metrics Integration Guide

This guide covers the comprehensive metrics refactoring that adds 9 new metrics modules to the existing analytics system.

## Overview

The Bongas AI analytics system has been expanded from 2 to **11 total metrics modules**, providing complete observability across all system components:

### Existing Modules (2)
- `kafka.rs` - Kafka producer/consumer metrics
- `clickhouse.rs` - Database query metrics

### New Modules (9)
- `cache.rs` - Redis and PostgreSQL L2 cache metrics
- `pipeline.rs` - Pipeline stage execution metrics
- `database.rs` - Repository operation metrics
- `middleware.rs` - HTTP middleware metrics
- `security.rs` - License and security validation metrics
- `experiment.rs` - Bandit and A/B test metrics
- `model.rs` - ML model lifecycle metrics
- `scenario.rs` - Scenario factory metrics
- `consumer.rs` - Kafka consumer metrics

## Architecture

### Central Registry Pattern

All metrics modules follow the same pattern:

```rust
pub struct ModuleMetrics {
    pub metric_name: IntCounterVec,
    pub metric_latency: HistogramVec,
    // ... more metrics
}

impl ModuleMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        // Register metrics with Prometheus registry
    }
}
```

### Analytics Manager Integration

The `AnalyticsManager` now includes all 11 modules:

```rust
pub struct AnalyticsManager {
    pub kafka: KafkaMetrics,
    pub db: ClickHouseMetrics,
    pub cache: CacheMetrics,
    pub pipeline: PipelineMetrics,
    pub database: DatabaseMetrics,
    pub middleware: MiddlewareMetrics,
    pub security: SecurityMetrics,
    pub experiment: ExperimentMetrics,
    pub model: ModelMetrics,
    pub scenario: ScenarioMetrics,
    pub consumer: ConsumerMetrics,
}
```

## Usage Examples

### Cache Metrics

```rust
use crate::analytics::ANALYTICS;

// Record cache hit
ANALYTICS.cache.l1_hits.with_label_values(&["redis"]).inc();

// Record cache lookup latency
let timer = ANALYTICS.cache.l1_lookup_latency
    .with_label_values(&["redis"])
    .start_timer();
// ... cache operation ...
timer.observe_duration();
```

### Pipeline Stage Metrics

```rust
// Record stage execution
ANALYTICS.pipeline.stage_executions
    .with_label_values(&["fetch_trending"])
    .inc();

// Record stage latency
let timer = ANALYTICS.pipeline.stage_execution_latency
    .with_label_values(&["fetch_trending"])
    .start_timer();
// ... stage execution ...
timer.observe_duration();
```

### Database Metrics

```rust
// Record repository operation
ANALYTICS.database.repository_operations
    .with_label_values(&["user_repo", "get_user"])
    .inc();

// Record query execution
let timer = ANALYTICS.database.query_execution_latency
    .with_label_values(&["users", "select"])
    .start_timer();
// ... query execution ...
timer.observe_duration();
```

### Security Metrics

```rust
// Record license validation
ANALYTICS.security.license_validations
    .with_label_values(&["license_key_check"])
    .inc();

// Record hardware fingerprinting
let timer = ANALYTICS.security.hardware_fingerprinting_latency
    .with_label_values(&["cpu"])
    .start_timer();
// ... fingerprinting ...
timer.observe_duration();
```

### Model Metrics

```rust
// Record model inference
ANALYTICS.model.model_inferences
    .with_label_values(&["two_tower"])
    .inc();

// Record inference latency
let timer = ANALYTICS.model.model_inference_latency
    .with_label_values(&["two_tower"])
    .start_timer();
// ... inference ...
timer.observe_duration();
```

## Metrics Labels

The system uses standardized labels for consistency:

- `cache_type` - "redis", "postgres"
- `scenario` - Scenario slug
- `stage` - Pipeline stage name
- `model_name` - Model identifier
- `middleware` - Middleware name
- `status_code` - HTTP status codes
- `error_type` - Error categories
- `experiment_id` - Experiment identifier
- `bandit_algorithm` - "ucb", "thompson_sampling", etc.
- `repository` - Repository name
- `operation` - Operation type
- `security_check` - Security check type
- `license_status` - "valid", "expired", "revoked"

## Prometheus Metrics

### Counter Metrics (Total counts)
- `bongas_cache_l1_hits_total`
- `bongas_pipeline_stage_executions_total`
- `bongas_database_repository_operations_total`
- `bongas_security_license_validations_total`
- `bongas_model_inferences_total`

### Histogram Metrics (Latency distributions)
- `bongas_cache_l1_lookup_duration_seconds`
- `bongas_pipeline_stage_duration_seconds`
- `bongas_database_query_duration_seconds`
- `bongas_security_license_validation_duration_seconds`
- `bongas_model_inference_duration_seconds`

### Gauge Metrics (Current values)
- `bongas_cache_connection_pool_size`
- `bongas_model_memory_usage_bytes`
- `bongas_model_cpu_usage_percent`

## Integration Points

### Cache Operations
- Redis operations in `src/cache/redis.rs`
- PostgreSQL operations in `src/cache/postgres_cache.rs`
- Cache warming in `src/cache/warming.rs`
- Cache invalidation in `src/engine/staging_manager.rs`

### Pipeline Stages
- All 40+ pipeline stages in `src/pipeline/stages/`
- Stage execution timing and success/failure tracking
- ONNX model inference metrics

### Database Operations
- All repository operations in `src/db/repositories/`
- Query execution timing
- Connection pool metrics
- Transaction metrics

### Middleware Operations
- HTTP request/response metrics
- Rate limiting effectiveness
- Compression efficiency
- CORS policy enforcement
- Error handling frequency

### Security Operations
- License validation success/failure
- Anti-debugging detection
- Hardware fingerprinting
- Binary integrity checks
- Security heartbeat monitoring

### ML Model Operations
- Model loading and deployment
- Inference latency and accuracy
- Feature extraction performance
- Model cache hit/miss rates

### Experiment Operations
- Bandit algorithm selections
- A/B test assignments and conversions
- Scenario performance tracking
- Reward processing latency

## Monitoring and Alerting

### Key Metrics to Monitor

#### Cache Health
- Cache hit rates (should be >80%)
- Cache lookup latency (should be <10ms)
- Cache warming success rate

#### Pipeline Performance
- Stage execution latency (should be <1s)
- Stage failure rates (should be <1%)
- ONNX inference latency (should be <100ms)

#### Database Performance
- Query execution latency (should be <500ms)
- Connection pool utilization
- Transaction failure rates

#### Security Health
- License validation success rate (should be 100%)
- Security violation frequency
- Anti-debugging detection events

#### Model Performance
- Model inference latency (should be <100ms)
- Model accuracy trends
- Feature extraction success rates

### Alerting Rules

```yaml
# Cache alerts
- alert: CacheHitRateLow
  expr: rate(bongas_cache_l1_hits_total[5m]) / (rate(bongas_cache_l1_hits_total[5m]) + rate(bongas_cache_l1_misses_total[5m])) < 0.8
  for: 5m

# Pipeline alerts
- alert: PipelineStageLatencyHigh
  expr: histogram_quantile(0.95, rate(bongas_pipeline_stage_duration_seconds_bucket[5m])) > 1.0
  for: 2m

# Database alerts
- alert: QueryLatencyHigh
  expr: histogram_quantile(0.95, rate(bongas_database_query_duration_seconds_bucket[5m])) > 0.5
  for: 2m

# Security alerts
- alert: LicenseValidationFailures
  expr: rate(bongas_security_license_validation_failures_total[5m]) > 0
  for: 1m

# Model alerts
- alert: ModelInferenceLatencyHigh
  expr: histogram_quantile(0.95, rate(bongas_model_inference_duration_seconds_bucket[5m])) > 0.1
  for: 2m
```

## Future Enhancements

### Planned Metrics
- Memory usage tracking across components
- CPU utilization per module
- Network I/O metrics
- Disk I/O metrics for ClickHouse
- Custom business metrics (recommendation quality, user engagement)

### Dashboard Templates
- Grafana dashboard configurations
- Prometheus recording rules
- Alert manager configurations
- SLI/SLO definitions

## Best Practices

### Metric Naming
- Use consistent prefixes (`bongas_`)
- Use descriptive metric names
- Follow Prometheus naming conventions
- Use appropriate metric types (Counter, Histogram, Gauge)

### Label Usage
- Use standardized label names
- Limit label cardinality
- Use meaningful label values
- Group related metrics with common labels

### Performance Considerations
- Minimize metric registration overhead
- Use appropriate bucket sizes for histograms
- Avoid high-cardinality labels
- Consider metric retention policies

## Troubleshooting

### Common Issues
- Missing label values in metric registration
- Incorrect metric types for use cases
- High cardinality causing performance issues
- Metric name conflicts

### Debugging Tips
- Use Prometheus query console to verify metrics
- Check metric registration in application logs
- Monitor metric collection intervals
- Validate label consistency across modules

## Migration Guide

### From Old System
The existing `kafka` and `clickhouse` metrics continue to work unchanged. The new modules are additive and don't break existing functionality.

### Adding New Metrics
1. Define metrics in appropriate module
2. Register metrics in module constructor
3. Add to AnalyticsManager struct
4. Update constructor to initialize new module
5. Add usage in relevant code locations

### Backward Compatibility
All existing metric names and labels remain unchanged. New metrics use the same naming conventions for consistency.