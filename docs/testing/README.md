# BONGAS-AI Testing Documentation

This document provides comprehensive information about the testing infrastructure for the BONGAS-AI recommendation system.

## Overview

The BONGAS-AI testing suite is designed to ensure the reliability, performance, and correctness of the recommendation system through multiple layers of testing:

- **Unit Tests**: Test individual components in isolation
- **Integration Tests**: Test component interactions and end-to-end workflows
- **API Tests**: Test REST API endpoints and functionality
- **Performance Tests**: Benchmark critical performance paths
- **Load Tests**: Validate system behavior under high load
- **Security Tests**: Verify security best practices

## Test Structure

```
tests/
├── common/              # Shared test utilities and fixtures
├── unit/               # Unit tests for individual components
│   ├── pipeline_stage_tests.rs
│   └── ml_tests.rs
├── integration/        # Integration tests
│   ├── end_to_end_tests.rs
│   └── api_tests.rs
└── cache/             # Cache-specific tests
    └── staging_manager_test.rs

benches/              # Performance benchmarks
├── pipeline_bench.rs
└── simple_bench.rs

scripts/
└── run_tests.sh      # Test runner script
```

## Test Configuration

### Test Environment Setup

The test suite uses a dedicated test configuration file:

- **Location**: `config/test.toml`
- **Purpose**: Database connections, Redis settings, ClickHouse configuration
- **Environment**: Isolated from production and development environments

### Test Database

- **Type**: PostgreSQL with test-specific schema
- **Setup**: Automated migration and seeding
- **Cleanup**: Automatic cleanup between tests
- **Data**: Synthetic test data with realistic patterns

### Test Services

The following services are required for integration testing:

```bash
# Start test services
docker-compose up -d

# Services include:
# - PostgreSQL (test database)
# - Redis (test cache)
# - ClickHouse (test analytics)
# - Kafka (test message queue)
```

## Running Tests

### Quick Start

```bash
# Run all tests
./scripts/run_tests.sh

# Run specific test types
./scripts/run_tests.sh unit
./scripts/run_tests.sh integration
./scripts/run_tests.sh bench
./scripts/run_tests.sh coverage
```

### Manual Test Execution

```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration

# API tests
cargo test --test api_tests

# All tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_pipeline_stage_execution
```

### Performance Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench bench_pipeline_stages

# Generate benchmark reports
cargo bench --bench simple_bench
```

## Test Coverage

### Coverage Targets

- **Unit Test Coverage**: > 80% line coverage
- **Integration Test Coverage**: All critical workflows
- **API Test Coverage**: All REST endpoints
- **Performance Test Coverage**: All pipeline stages

### Coverage Reporting

```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# View report
open coverage/tarpaulin-report.html
```

## Test Categories

### 1. Unit Tests

**Location**: `tests/unit/`

**Purpose**: Test individual components in isolation

**Coverage**:
- Pipeline stages (ONNX inference, filtering, diversification)
- Cache operations (Redis, PostgreSQL)
- ML components (feature extraction, bandit algorithms)
- ONNX runtime functionality
- Error handling and edge cases

**Example**:
```rust
#[tokio::test]
async fn test_onnx_inference_stage() -> Result<()> {
    let stage = ONNXInferenceStage;
    let context = create_test_context();
    let items = create_test_items(100);
    
    let result = stage.execute(&context, &params, items).await?;
    
    assert_eq!(result.len(), 100);
    assert!(result.iter().all(|item| item.score.is_finite()));
    
    Ok(())
}
```

### 2. Integration Tests

**Location**: `tests/integration/`

**Purpose**: Test component interactions and end-to-end workflows

**Coverage**:
- Complete pipeline execution
- Scenario hot-reload functionality
- Cache warming and invalidation
- Staleness detection and handling
- Concurrent operations
- Error recovery

**Example**:
```rust
#[tokio::test]
async fn test_complete_pipeline_execution() -> Result<()> {
    let engine = setup_test_engine().await?;
    let context = create_test_context();
    
    let result = engine.execute_scenario("trending_now", 123, context).await?;
    
    assert!(result.is_ok());
    assert!(result.unwrap().len() > 0);
    
    Ok(())
}
```

### 3. API Tests

**Location**: `tests/integration/api_tests.rs`

**Purpose**: Test REST API endpoints and functionality

**Coverage**:
- Scenario CRUD operations
- Recommendation endpoints
- Cache management endpoints
- Error handling and validation
- Authentication (when implemented)

**Example**:
```rust
#[tokio::test]
async fn test_get_recommendations() -> Result<()> {
    let app = create_test_app().await?;
    
    let request = Request::builder()
        .uri("/api/v1/recommendations/trending_now/123")
        .method("GET")
        .body(Body::empty())?;
    
    let response = app.oneshot(request).await?;
    
    assert_eq!(response.status(), StatusCode::OK);
    
    Ok(())
}
```

### 4. Performance Tests

**Location**: `benches/`

**Purpose**: Benchmark critical performance paths

**Coverage**:
- Pipeline stage execution times
- Cache operation performance
- ML component performance
- Concurrent operation throughput
- Memory usage patterns
- End-to-end pipeline latency

**Performance Targets**:
- P95 Latency (cached): < 100ms
- P95 Latency (uncached): < 300ms
- Throughput: > 10,000 req/s
- Cache Hit Rate: > 85%
- Hot-Reload Time: < 1 second

## Test Data

### Synthetic Data Generation

The test suite uses synthetic data that mimics real-world patterns:

- **Users**: 10,000+ test users with diverse profiles
- **Items**: 50,000+ test items with metadata
- **Interactions**: Realistic viewing patterns and preferences
- **Features**: Generated user and item features for ML testing

### Data Seeding

```rust
// Seed test data
async fn seed_test_data(pool: &PgPool) -> Result<()> {
    // Create test users
    for i in 0..10000 {
        create_test_user(pool, i).await?;
    }
    
    // Create test items
    for i in 0..50000 {
        create_test_item(pool, i).await?;
    }
    
    // Create interactions
    create_test_interactions(pool).await?;
    
    Ok(())
}
```

## Continuous Integration

### CI Pipeline

The test suite is designed for CI/CD integration:

```yaml
# Example GitHub Actions workflow
- name: Run Tests
  run: ./scripts/run_tests.sh ci

- name: Generate Coverage
  run: ./scripts/run_tests.sh coverage

- name: Run Benchmarks
  run: ./scripts/run_tests.sh bench
```

### CI Test Types

- **Fast Tests**: Unit tests (run on every commit)
- **Full Tests**: All tests (run on pull requests)
- **Performance Tests**: Benchmarks (run nightly)
- **Load Tests**: Stress testing (run weekly)

## Debugging Tests

### Common Issues

1. **Database Connection Issues**
   ```bash
   # Check if test database is running
   docker-compose ps
   
   # Reset test database
   docker-compose down && docker-compose up -d
   ```

2. **Redis Connection Issues**
   ```bash
   # Check Redis status
   redis-cli ping
   
   # Clear Redis cache
   redis-cli flushall
   ```

3. **Test Failures**
   ```bash
   # Run failing test with verbose output
   cargo test failing_test_name -- --nocapture --show-output
   ```

### Debug Tools

- **Test Logging**: All tests include detailed logging
- **Performance Profiling**: Built-in timing measurements
- **Memory Analysis**: Memory usage tracking in benchmarks
- **Error Context**: Rich error messages with context

## Best Practices

### Writing Tests

1. **Isolation**: Each test should be independent
2. **Deterministic**: Tests should produce consistent results
3. **Fast**: Tests should run quickly (< 1s for unit tests)
4. **Clear**: Test names should describe what is being tested
5. **Comprehensive**: Cover both happy path and edge cases

### Test Organization

```rust
mod pipeline_stage_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_onnx_inference_stage() -> Result<()> {
        // Test implementation
    }
    
    #[tokio::test]
    async fn test_onnx_inference_stage_with_invalid_input() -> Result<()> {
        // Edge case testing
    }
}
```

### Performance Testing

1. **Baseline Measurements**: Establish performance baselines
2. **Regression Detection**: Monitor for performance regressions
3. **Load Testing**: Test under realistic load
4. **Memory Profiling**: Monitor memory usage patterns

## Monitoring and Metrics

### Test Metrics

The test suite collects various metrics:

- **Test Execution Time**: Track test performance
- **Coverage Metrics**: Monitor code coverage
- **Failure Rates**: Track test reliability
- **Performance Benchmarks**: Monitor system performance

### Reporting

- **Test Reports**: Detailed HTML reports
- **Coverage Reports**: Line-by-line coverage analysis
- **Performance Reports**: Benchmark comparison charts
- **CI Integration**: Automated reporting in CI/CD

## Security Testing

### Security Validation

The test suite includes security checks:

- **Secret Detection**: Scan for hardcoded secrets
- **Input Validation**: Test malicious input handling
- **Error Information**: Verify no sensitive data in errors
- **Resource Limits**: Test resource exhaustion scenarios

### Security Best Practices

- **Test Data**: Use synthetic data, never production data
- **Credentials**: Use test-only credentials
- **Network**: Isolate test networks from production
- **Access**: Limit test environment access

## Troubleshooting

### Common Problems

1. **Tests Running Slow**
   - Check if test services are running locally
   - Verify database performance
   - Check for resource contention

2. **Intermittent Failures**
   - Check for race conditions in concurrent tests
   - Verify test isolation
   - Review timing dependencies

3. **Coverage Issues**
   - Ensure all code paths are tested
   - Check for unreachable code
   - Review conditional logic coverage

### Getting Help

- **Documentation**: Check this README and inline comments
- **Issues**: Report bugs in the project issue tracker
- **Community**: Ask questions in project discussions
- **Code Review**: Request reviews for complex test changes

## Future Enhancements

### Planned Improvements

1. **Property-Based Testing**: Add QuickCheck-style testing
2. **Fuzz Testing**: Add fuzz testing for input validation
3. **Chaos Engineering**: Add chaos testing for resilience
4. **Contract Testing**: Add API contract testing
5. **Golden Master Testing**: Add snapshot testing for outputs

### Performance Optimization

1. **Parallel Test Execution**: Optimize test parallelization
2. **Test Data Optimization**: Improve test data generation
3. **Resource Management**: Better resource cleanup
4. **CI Optimization**: Faster CI pipeline execution

## Conclusion

The BONGAS-AI testing infrastructure provides comprehensive coverage of the recommendation system through multiple testing layers. This ensures the system's reliability, performance, and correctness in production environments.

For questions or issues related to testing, please refer to the project documentation or contact the development team.