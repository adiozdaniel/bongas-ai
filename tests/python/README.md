# BONGAS-AI Python Test Suite

This directory contains comprehensive test suites for the BONGAS-AI Python components, including ML models, training pipelines, ONNX export functionality, and performance benchmarks.

## Test Structure

```
tests/python/
├── README.md                    # This file
├── run_tests.py                # Test runner script
├── pytest.ini                  # Pytest configuration
├── requirements.txt              # Test dependencies
├── fixtures/                     # Test utilities and fixtures
│   └── __init__.py
├── unit/                        # Unit tests
│   ├── __init__.py
│   ├── test_models.py           # ML model tests
│   ├── test_training.py         # Training pipeline tests
│   └── test_export.py           # ONNX export tests
├── integration/                  # Integration tests
│   ├── __init__.py
│   └── test_training_pipeline.py # End-to-end workflow tests
├── performance/                  # Performance tests
│   ├── __init__.py
│   └── test_model_inference.py   # Inference performance tests
└── reports/                     # Test reports (generated)
```

## Test Categories

### 1. Unit Tests (`tests/python/unit/`)

**Purpose**: Test individual components in isolation

**Coverage**:
- **Models**: Two-Tower model functionality, forward passes, embeddings
- **Training**: Dataset handling, training loops, validation
- **Export**: ONNX export, model conversion, accuracy validation

**Key Features**:
- Mock external dependencies
- Fast execution
- Comprehensive edge case coverage
- Parameter validation

### 2. Integration Tests (`tests/python/integration/`)

**Purpose**: Test component interactions and end-to-end workflows

**Coverage**:
- Complete training pipeline from data to model
- Training + export workflow integration
- Realistic data scenarios
- Error handling across components

**Key Features**:
- Real data flow testing
- Cross-component validation
- Performance monitoring
- Memory usage validation

### 3. Performance Tests (`tests/python/performance/`)

**Purpose**: Validate performance characteristics and scalability

**Coverage**:
- Inference latency and throughput
- Memory usage patterns
- PyTorch vs ONNX performance comparison
- Concurrent inference testing
- Stress testing and stability

**Key Features**:
- Benchmarking capabilities
- Memory leak detection
- Long-duration stability tests
- Extreme batch size handling

## Test Configuration

### Pytest Configuration (`pytest.ini`)

The test suite uses pytest with the following key configurations:

- **Coverage**: 80% minimum coverage requirement
- **Parallel execution**: Support for `pytest-xdist`
- **Reporting**: HTML and JSON reports
- **Timeouts**: 300-second test timeout
- **Markers**: Organized test categorization

### Test Markers

- `@pytest.mark.unit`: Unit tests
- `@pytest.mark.integration`: Integration tests
- `@pytest.mark.performance`: Performance tests
- `@pytest.mark.slow`: Slow-running tests
- `@pytest.mark.gpu`: GPU-dependent tests
- `@pytest.mark.onnx`: ONNX-related tests

## Running Tests

### Prerequisites

Install test dependencies:
```bash
python tests/python/run_tests.py --install
```

### Test Runner Script

Use the provided test runner for convenient test execution:

```bash
# Show test summary and usage
python tests/python/run_tests.py --summary

# Run specific test suites
python tests/python/run_tests.py --unit
python tests/python/run_tests.py --integration
python tests/python/run_tests.py --performance

# Run all tests
python tests/python/run_tests.py --all

# Generate coverage report
python tests/python/run_tests.py --coverage

# Run specific test file
python tests/python/run_tests.py --file tests/python/unit/test_models.py

# Clean reports
python tests/python/run_tests.py --clean
```

### Direct Pytest Usage

You can also run tests directly with pytest:

```bash
# Run all tests
pytest tests/python/

# Run specific category
pytest tests/python/ -m unit
pytest tests/python/ -m integration
pytest tests/python/ -m performance

# Run with coverage
pytest tests/python/ --cov=bongas_ml --cov-report=html

# Run in parallel
pytest tests/python/ -n auto

# Run with verbose output
pytest tests/python/ -v
```

## Test Fixtures and Utilities

### TestConfig Class

Centralized test configuration in `fixtures/__init__.py`:

```python
class TestConfig:
    USER_FEATURE_DIM = 64
    ITEM_FEATURE_DIM = 32
    EMBEDDING_DIM = 128
    HIDDEN_DIMS = [256, 128]
    BATCH_SIZE = 32
    LEARNING_RATE = 1e-3
    EPOCHS = 2
    NUM_USERS = 100
    NUM_ITEMS = 50
    NUM_INTERACTIONS = 1000
    ONNX_OPSET_VERSION = 17
```

### Helper Functions

- `create_synthetic_user_features()`: Generate synthetic user data
- `create_synthetic_item_features()`: Generate synthetic item data
- `create_synthetic_interactions()`: Create interaction datasets
- `get_test_device()`: Get best available device (CPU/GPU)
- `assert_tensors_close()`: Compare tensor values with tolerance
- `assert_shapes_match()`: Validate tensor shapes

## Performance Benchmarks

### Target Performance Metrics

**Inference Latency**:
- Single inference: < 0.1ms (PyTorch), < 0.2ms (ONNX)
- Batch size 32: < 0.5ms (PyTorch), < 1.0ms (ONNX)

**Throughput**:
- Batch size 32: > 1000 predictions/second
- Batch size 128: > 2000 predictions/second
- Large batches (1000): > 5000 predictions/second

**Memory Usage**:
- Linear scaling with batch size
- No memory leaks during extended operation
- Reasonable memory overhead (< 3x expected)

### Performance Testing Features

- **Warmup validation**: Ensures models are properly warmed up
- **Concurrent testing**: Validates multi-GPU and multi-model scenarios
- **Precision comparison**: FP32 vs FP16 performance analysis
- **Stress testing**: Long-duration and extreme batch size validation

## Coverage Requirements

### Minimum Coverage: 80%

Coverage is enforced with the following breakdown:
- **Models**: 90%+ coverage
- **Training**: 85%+ coverage  
- **Export**: 85%+ coverage
- **Integration**: 75%+ coverage
- **Performance**: 70%+ coverage

### Coverage Reports

Generated reports include:
- **HTML**: Interactive coverage browser
- **XML**: CI/CD integration
- **Terminal**: Missing line highlights

## Continuous Integration

### GitHub Actions Integration

The test suite is designed for CI/CD with:
- Parallel test execution
- Coverage reporting
- Performance regression detection
- Multi-platform testing

### Example Workflow

```yaml
name: Python Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.10'
      - run: python tests/python/run_tests.py --install
      - run: python tests/python/run_tests.py --all
      - run: python tests/python/run_tests.py --coverage
```

## Best Practices

### Writing Tests

1. **Use appropriate markers**: Categorize tests correctly
2. **Mock external dependencies**: Use `@patch` for external services
3. **Test edge cases**: Include boundary conditions and error scenarios
4. **Use fixtures**: Leverage shared test utilities
5. **Validate performance**: Include timing assertions for critical paths

### Performance Testing

1. **Warmup models**: Always warm up before measuring
2. **Multiple runs**: Use statistical analysis for timing
3. **Memory monitoring**: Track memory usage patterns
4. **Realistic data**: Use synthetic data that mimics production
5. **Document assumptions**: Note hardware dependencies

### Debugging Tests

1. **Verbose output**: Use `-v` flag for detailed output
2. **Specific files**: Run individual test files for focused debugging
3. **Coverage reports**: Use HTML coverage for line-by-line analysis
4. **Performance profiling**: Use `pytest-benchmark` for performance issues

## Troubleshooting

### Common Issues

**Import Errors**:
```bash
# Ensure project root is in Python path
export PYTHONPATH=/path/to/bongas-ai:$PYTHONPATH
```

**GPU Issues**:
```bash
# Skip GPU tests if no CUDA available
pytest tests/python/ -m "not gpu"
```

**Memory Issues**:
```bash
# Run tests sequentially to reduce memory usage
pytest tests/python/ -n 1
```

**Performance Regression**:
```bash
# Compare with baseline performance
pytest tests/python/performance/ --benchmark-compare
```

### Getting Help

- Check test logs in `tests/python/reports/`
- Review coverage reports for untested code paths
- Use `--verbose` flag for detailed test output
- Run specific test categories to isolate issues

## Contributing

When adding new tests:

1. **Follow naming conventions**: `test_*.py` files, `Test*` classes, `test_*` methods
2. **Add appropriate markers**: Use existing markers or create new ones
3. **Include fixtures**: Use or extend existing test utilities
4. **Update documentation**: Add test descriptions and usage examples
5. **Validate performance**: Ensure new tests don't significantly slow down the suite

## Test Data

### Synthetic Data Generation

All tests use synthetic data to ensure:
- **Reproducibility**: Consistent test results
- **Performance**: Fast test execution
- **Privacy**: No real user data exposure
- **Coverage**: Edge cases and boundary conditions

### Data Structure

```python
interaction = {
    'user_id': int,
    'item_id': int,
    'user_features': np.ndarray,  # Shape: (USER_FEATURE_DIM,)
    'item_features': np.ndarray,  # Shape: (ITEM_FEATURE_DIM,)
    'rating': float,
    'timestamp': int
}
```

## Future Enhancements

### Planned Improvements

1. **Property-based testing**: Using Hypothesis for more comprehensive testing
2. **Fuzzing**: Input validation and edge case discovery
3. **Model validation**: Automated model correctness verification
4. **Performance baselines**: Historical performance tracking
5. **Cross-platform testing**: Windows, macOS, Linux validation

### Advanced Testing

- **Distributed training tests**: Multi-GPU and multi-node scenarios
- **Model serving tests**: Integration with serving infrastructure
- **A/B testing framework**: Model comparison and validation
- **Data drift detection**: Automated data quality monitoring

---

For more information, see the individual test files and the main project documentation.