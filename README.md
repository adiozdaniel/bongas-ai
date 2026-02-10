# Recommendation System Implementation Documentation

This repository contains comprehensive documentation for implementing a modern, production-ready recommendation system. The documentation is organized into architectural decision records (ADRs), implementation phases, and a detailed implementation guide.

## 📋 Table of Contents

- [📋 Table of Contents](#-table-of-contents)
- [🏗️ Architecture Overview](#️-architecture-overview)
- [📚 Documentation Structure](#-documentation-structure)
- [🚀 Quick Start](#-quick-start)
- [📖 Implementation Phases](#-implementation-phases)
- [🔧 Development Setup](#-development-setup)
- [🧪 Testing](#-testing)
- [📊 Monitoring & Observability](#-monitoring--observability)
- [🔒 Security](#-security)
- [📈 Performance](#-performance)
- [🏗️ Production Deployment](#️-production-deployment)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)
- [📖 Detailed Guides](#-detailed-guides)

## 📖 Detailed Guides

| Guide                                             | Description                                                  |
| ------------------------------------------------- | ------------------------------------------------------------ |
| [Architecture](docs/guides/architecture.md)       | System diagram, core components, data flow, technology stack |
| [Scenarios](docs/guides/scenarios.md)             | JSONB pipeline system, stage catalog, hot-reload, CRUD       |
| [ML Integration](docs/guides/ml_integration.md)   | Training workflow, ONNX inference, feature store, bandits    |
| [ONNX Deployment](docs/guides/onnx_deployment.md) | Model lifecycle: train, export, validate, deploy, serve      |
| [API Reference](docs/guides/api_reference.md)     | All REST endpoints with request/response examples            |

## 🏗️ Architecture Overview

The recommendation system follows a modern microservices architecture with the following key components:

### Core Architecture

- **Rust Backend**: High-performance API service with async capabilities
- **Python ML Service**: Machine learning models and algorithms
- **PostgreSQL**: Primary database for user data and interactions
- **Redis**: Caching layer for performance optimization
- **Kubernetes**: Container orchestration for scalability
- **Frontend**: React-based user interface

### Key Features

- **Multi-algorithm Support**: Collaborative filtering, content-based, hybrid approaches
- **Real-time Recommendations**: Sub-100ms response times
- **A/B Testing**: Built-in experimentation framework
- **Scalable Architecture**: Horizontal scaling capabilities
- **Production Ready**: Comprehensive monitoring and observability

## 📚 Documentation Structure

---

BONGAS 3.0 - Complete New Project Structure

```text
bongas-ai/ # ← NEW standalone project
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .env.example
├── Dockerfile
│
├── migrations/ # PostgreSQL migrations
│ ├── 001_init_schema.sql
│ ├── 002_scenario_configs.sql
│ ├── 003_seed_scenarios.sql
│ ├── 004_feature_store.sql
│ ├── 005_staging_cache.sql
│ └── 006_experiments.sql
│
├── config/ # Configuration files
│ ├── default.toml
│ └── scenarios.example.json
│
├── models/ # ✅ ONNX model files (production)
│ ├── two_tower_v1.onnx
│ ├── bert4rec_v1.onnx
│ ├── ncf_v1.onnx
│ ├── din_v1.onnx
│ └── README.md # Model versioning info
│
├── src/
│ ├── main.rs # Application entry point
│ ├── lib.rs
│ │
│ ├── api/ # ✅ API v1 endpoints
│ │ ├── mod.rs
│ │ ├── v1/
│ │ │ ├── mod.rs
│ │ │ ├── recommendations.rs # Dynamic recommendation endpoints
│ │ │ ├── scenarios.rs # Scenario CRUD endpoints
│ │ │ ├── experiments.rs # A/B testing endpoints
│ │ │ ├── features.rs # Feature store endpoints
│ │ │ └── analytics.rs # Real-time analytics
│ │ ├── middleware.rs
│ │ └── error.rs
│ │
│ ├── bongas/ # Core BONGAS engine
│ │ ├── mod.rs
│ │ ├── engine.rs # Main orchestrator
│ │ ├── staging_manager.rs # L2 cache layer
│ │ ├── staleness_engine.rs # Behavior-aware invalidation
│ │ ├── context.rs # Request context types
│ │ └── config.rs # BONGAS configuration
│ │
│ ├── scenarios/ # Scenario system
│ │ ├── mod.rs
│ │ ├── traits.rs # ScenarioStrategy trait
│ │ ├── factory.rs # Creates scenarios from DB
│ │ ├── loader.rs # Hot-reload from database
│ │ │
│ │ └── dynamic/ # Data-driven scenarios
│ │ ├── mod.rs
│ │ ├── pipeline_executor.rs # Executes JSONB pipelines
│ │ ├── stages/ # Composable pipeline stages
│ │ │ ├── mod.rs
│ │ │ ├── collaborative_filtering.rs
│ │ │ ├── content_based.rs
│ │ │ ├── hybrid.rs
│ │ │ ├── filters.rs # Genre, age, etc.
│ │ │ ├── boosters.rs # Trending, new content
│ │ │ ├── diversifiers.rs # Diversity, serendipity
│ │ │ └── onnx_stages.rs # ✅ ONNX inference stages
│ │ └── dynamic_scenario.rs # Dynamic scenario impl
│ │
│ ├── ml/ # ✅ ML infrastructure (ONNX-based)
│ │ ├── mod.rs
│ │ ├── onnx_runtime.rs # ONNX Runtime wrapper
│ │ ├── model_loader.rs # Load .onnx files
│ │ ├── inference.rs # Batch/online inference
│ │ ├── preprocessing.rs # Feature preprocessing (Rust-native)
│ │ ├── postprocessing.rs # Score normalization, ranking
│ │ ├── feature_store.rs # Centralized features
│ │ ├── model_registry.rs # Versioned model storage
│ │ ├── embeddings.rs # User/item embeddings
│ │ ├── online_learning.rs # Real-time model updates
│ │ └── worker_queue.rs # Background ML tasks
│ │
│ ├── experiments/ # Experimentation framework
│ │ ├── mod.rs
│ │ ├── manager.rs # Experiment orchestration
│ │ ├── bandits/ # ✅ Native Rust implementation
│ │ │ ├── mod.rs
│ │ │ ├── thompson_sampling.rs # Bayesian bandits
│ │ │ ├── ucb.rs # Upper Confidence Bound
│ │ │ ├── linucb.rs # Contextual bandits
│ │ │ └── epsilon_greedy.rs # Simple ε-greedy
│ │ └── ab_testing.rs # A/B/n testing
│ │
│ ├── db/ # Database layer
│ │ ├── mod.rs
│ │ ├── repositories/
│ │ │ ├── mod.rs
│ │ │ ├── scenario_repository.rs
│ │ │ ├── feature_repository.rs
│ │ │ ├── configuration_repository.rs
│ │ │ ├── user_repository.rs # User-device tracking
│ │ │ ├── interaction_repository.rs
│ │ │ └── experiment_repository.rs
│ │ └── models.rs
│ │
│ ├── cache/ # Caching infrastructure
│ │ ├── mod.rs
│ │ ├── redis.rs # L1 cache
│ │ ├── postgres_cache.rs # L2 cache
│ │ └── strategies.rs # Eviction policies
│ │
│ ├── kafka/ # Event consumers
│ │ ├── mod.rs
│ │ ├── profile_consumer.rs
│ │ ├── reaction_consumer.rs
│ │ ├── notification_consumer.rs
│ │ ├── playback_consumer.rs
│ │ └── metrics.rs
│ │
│ ├── analytics/ # Real-time analytics
│ │ ├── mod.rs
│ │ ├── clickhouse.rs # ClickHouse integration
│ │ └── metrics.rs # Prometheus metrics
│ │
│ ├── security/ # Security layer (8-layer)
│ │ ├── mod.rs
│ │ ├── license.rs
│ │ ├── binary.rs # Binary Integrity
│ │ ├── manager.rs # Orchestrator
│ │ ├── anti_debug.rs # Debugging Detector
│ │ ├── validator.rs # Server-side validation
│ │ └── hardware.rs # Hardware fingerprinting
│ │
│ └── config/ # Configuration management
│ ├── mod.rs
│ ├── settings.rs
│ ├── kafka.rs
│ └── spring_cloud.rs
│
├── python/ # ✅ Training code (NOT SHIPPED)
│ ├── setup.py
│ ├── pyproject.toml
│ ├── requirements.txt
│ ├── README.md
│ │
│ └── bongas_ml/ # Python ML package (dev only)
│ ├── **init**.py
│ ├── **main**.py
│ │
│ ├── models/ # ML model definitions
│ │ ├── **init**.py
│ │ ├── base.py # Base model interface
│ │ ├── two_tower.py # Two-Tower model
│ │ ├── bert4rec.py # BERT4Rec transformer
│ │ ├── ncf.py # Neural Collaborative Filtering
│ │ ├── din.py # Deep Interest Network
│ │ ├── wide_and_deep.py # Wide & Deep
│ │ └── autoint.py # AutoInt (feature interactions)
│ │
│ ├── training/ # Training pipeline
│ │ ├── **init**.py
│ │ ├── trainer.py # Training orchestration
│ │ ├── datasets.py # PyTorch datasets
│ │ ├── losses.py # Custom loss functions
│ │ └── callbacks.py # Training callbacks
│ │
│ ├── export/ # ✅ ONNX export utilities
│ │ ├── **init**.py
│ │ ├── onnx_exporter.py # PyTorch → ONNX
│ │ ├── validate.py # Validate ONNX matches PyTorch
│ │ └── optimize.py # ONNX optimization
│ │
│ ├── features/ # Feature engineering
│ │ ├── **init**.py
│ │ ├── extractors.py # Feature extraction
│ │ ├── transformers.py # Feature transformation
│ │ └── embeddings.py # Embedding generation
│ │
│ └── utils/ # Utilities
│ ├── **init**.py
│ ├── metrics.py # Evaluation metrics
│ └── logging.py # Python logging
│
├── scripts/ # Build & deployment scripts
│ ├── train_and_export.sh # ✅ Train → Export ONNX
│ ├── validate_onnx.py # ✅ Validate ONNX models
│ └── package.sh # Package Rust binary + .onnx
│
├── tests/ # Integration tests
│ ├── api/
│ ├── scenarios/
│ ├── ml/
│ │ ├── onnx_inference_test.rs # ✅ Test ONNX Runtime
│ │ └── model_accuracy_test.rs # ✅ Compare ONNX vs PyTorch
│ └── bandits/
│
├── benches/ # Benchmarks
│ ├── pipeline_bench.rs
│ └── onnx_inference_bench.rs # ✅ ONNX inference benchmarks
│
└── docs/ # Documentation
├── README.md # This file
└── guides/ # Detailed reference guides
├── architecture.md # System design & data flow
├── scenarios.md # JSONB pipeline system
├── ml_integration.md # ML training & inference
├── onnx_deployment.md # ONNX model lifecycle
└── api_reference.md # REST API endpoints
```

---

Key Changes from PyO3 to ONNX

| Component      | PyO3 Approach              | ONNX Approach                  |
| -------------- | -------------------------- | ------------------------------ |
| Python Runtime | Embedded via PyO3 (~200MB) | ❌ Not needed                  |
| ML Inference   | Python code via FFI        | ✅ ONNX Runtime (C++)          |
| Model Files    | .so binaries (Cython)      | ✅ .onnx binaries              |
| Data Transfer  | Apache Arrow (zero-copy)   | ✅ Native tensors (faster)     |
| GIL Management | Complex release strategies | ✅ No GIL (no Python)          |
| Training       | Same Python codebase       | ✅ Same, but export step added |
| Bandits        | Python implementation      | ✅ Native Rust (better)        |
| Binary Size    | ~450MB (with Python)       | ✅ ~250MB (no Python)          |
| Performance    | Good (with GIL tricks)     | ✅ Excellent (native)          |
| IP Protection  | Cython → .so               | ✅ .onnx binary format         |

---

Production Workflow

┌─────────────────────────────────────────────────────────────────┐
│ DEVELOPMENT (Python) │
│ │
│ 1. Train models in PyTorch (python/bongas_ml/training/) │
│ 2. Validate accuracy │
│ 3. Export to ONNX (torch.onnx.export) │
│ 4. Validate ONNX output matches PyTorch │
│ 5. Optimize ONNX (onnxruntime.transformers.optimizer) │
│ │
└────────────────────────┬─────────────────────────────────────────┘
│
▼
┌─────────────────────────────────────────────────────────────────┐
│ PRODUCTION (Rust + ONNX) │
│ │
│ Single binary contains: │
│ - Rust application code │
│ - ONNX Runtime (statically linked) │
│ │
│ Shipped separately: │
│ - models/\*.onnx files │
│ │
│ NO Python runtime needed! │
│ │
└─────────────────────────────────────────────────────────────────┘

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+**: For the main application
- **Python 3.9+**: For ML components
- **PostgreSQL 14+**: Primary database
- **Redis 7+**: Caching layer
- **Docker 20+**: Containerization
- **Kubernetes 1.25+**: Orchestration (for production)
- **Node.js 18+**: For frontend components

### Installation

1. **Clone the repository:**

   ```bash
   git clone <repository-url>
   cd bongas-ai
   ```

2. **Install dependencies:**

   ```bash
   # Rust dependencies
   cargo install cargo-audit cargo-deny

   # Python dependencies
   cd ml_service && poetry install

   # Node.js dependencies
   cd frontend && npm install
   ```

3. **Set up environment:**

   ```bash
   # Copy environment templates
   cp .env.development.template .env.development
   cp ml_service/.env.development.template ml_service/.env.development
   cp frontend/.env.development.template frontend/.env.development

   # Configure your environment variables
   ```

4. **Start development environment:**

   ```bash
   # Start services with Docker Compose
   docker-compose up -d

   # Start Rust development server
   cargo run

   # Start Python ML service
   cd ml_service && poetry run python -m ml_service.main

   # Start frontend development server
   cd frontend && npm run dev
   ```

## 📖 Implementation Phases

The implementation is organized into 10 comprehensive phases:

### Phase 0: Foundation (1-2 days)

- Project scaffolding and structure
- Development environment setup
- Basic configuration and dependencies
- Initial CI/CD pipeline

### Phase 1: Database Layer (2-3 days)

- Database schema design and migrations
- Rust database models and repositories
- Connection pooling and optimization
- Data validation and constraints

### Phase 2: Configuration & Security (1-2 days)

- Configuration management system
- Security implementation (JWT, bcrypt)
- Environment-specific configurations
- Secret management

### Phase 3: API & Middleware (2-3 days)

- RESTful API design and implementation
- Middleware for logging, CORS, rate limiting
- Request/response validation
- Error handling and middleware stack

### Phase 4: Machine Learning Layer (3-4 days)

- ML model architecture and integration
- Feature engineering and preprocessing
- Model training and evaluation
- Python service implementation

### Phase 5: Python Bridge (2-3 days)

- Rust-Python integration
- Model serving and inference
- Data serialization and communication
- Performance optimization

### Phase 6: Scenario Pipeline (3-4 days)

- Algorithm selection and orchestration
- Feature extraction and processing
- Recommendation pipeline implementation
- Performance monitoring

### Phase 7: Staging & Caching (2-3 days)

- Staging environment setup
- Multi-level caching strategies
- Cache warming and invalidation
- Performance testing

### Phase 8: Home Feed & UI (3-4 days)

- Home feed service implementation
- User interaction tracking
- Real-time updates and notifications
- Frontend integration

### Phase 9: A/B Testing (2-3 days)

- Experiment management system
- Statistical analysis engine
- Dashboard and visualization
- Result tracking and analysis

### Phase 10: Production (4-5 days)

- Production Kubernetes deployment
- Monitoring and observability stack
- CI/CD pipeline for production
- Performance optimization and scaling

## 🔧 Development Setup

### Local Development

1. **Database Setup:**

   ```bash
   # Start PostgreSQL
   docker-compose up -d postgres

   # Run migrations
   cargo run --bin migrate

   # Seed initial data
   cargo run --bin seed
   ```

2. **Development Commands:**

   ```bash
   # Run tests
   cargo test
   cd ml_service && poetry run pytest
   cd frontend && npm test

   # Run linting
   cargo clippy
   cd ml_service && poetry run ruff check .
   cd frontend && npm run lint

   # Format code
   cargo fmt
   cd ml_service && poetry run ruff format .
   cd frontend && npm run format
   ```

3. **Development Workflow:**

   ```bash
   # Create feature branch
   git checkout -b feature/new-feature

   # Make changes and commit
   git add .
   git commit -m "Add new feature"

   # Push and create PR
   git push -u origin feature/new-feature
   ```

## 🧪 Testing

### Test Strategy

The project follows a comprehensive testing strategy:

1. **Unit Tests**: Test individual components and functions
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete user journeys
4. **Performance Tests**: Load and stress testing
5. **Security Tests**: Vulnerability scanning and security testing

### Running Tests

```bash
# Run all tests
cargo test
cd ml_service && poetry run pytest
cd frontend && npm test

# Run specific test suites
cargo test --test integration
cd ml_service && poetry run pytest tests/integration/
cd frontend && npm run test:integration

# Run performance tests
cargo test --release --features=performance
cd ml_service && poetry run pytest tests/performance/
```

## 📊 Monitoring & Observability

### Key Metrics

The system monitors various metrics across different layers:

#### Application Metrics

- Request rate and latency
- Error rates and types
- Cache hit rates
- Database connection pool usage
- ML model prediction latency

#### Business Metrics

- Recommendation click-through rate
- User engagement time
- Conversion rates
- A/B test performance

#### Infrastructure Metrics

- CPU and memory usage
- Disk space and I/O
- Network traffic
- Container restart rates

### Monitoring Stack

- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards
- **ELK Stack**: Log aggregation and analysis
- **AlertManager**: Alerting and notifications

### Setting Up Monitoring

```bash
# Deploy monitoring stack
kubectl apply -f monitoring/

# Access Grafana
kubectl port-forward -n monitoring svc/grafana 3000:80

# Access Prometheus
kubectl port-forward -n monitoring svc/prometheus 9090:9090
```

## 🔒 Security

### Security Features

- **Authentication**: JWT-based authentication with refresh tokens
- **Authorization**: Role-based access control (RBAC)
- **Input Validation**: Comprehensive request validation
- **Rate Limiting**: API rate limiting and DDoS protection
- **Security Headers**: OWASP-recommended security headers
- **Secret Management**: Secure secret storage and rotation

### Security Best Practices

1. **Environment Variables**: Never commit secrets to version control
2. **Input Validation**: Validate all user inputs
3. **Error Handling**: Don't expose sensitive information in errors
4. **Logging**: Secure logging without sensitive data
5. **Dependencies**: Regular security audits of dependencies

## 📈 Performance

### Performance Goals

- **Response Time**: 95th percentile < 100ms
- **Throughput**: 10,000+ requests/second
- **Availability**: 99.9% uptime
- **Scalability**: Horizontal scaling to handle load

### Performance Optimization

1. **Database Optimization**:
   - Proper indexing strategies
   - Connection pooling
   - Query optimization

2. **Caching Strategies**:
   - Multi-level caching (memory, Redis, database)
   - Smart cache invalidation
   - Cache warming

3. **ML Model Optimization**:
   - Model caching and warmup
   - Batch processing
   - Efficient feature extraction

4. **Infrastructure Optimization**:
   - Container resource limits
   - Load balancing
   - CDN for static assets

## 🏗️ Production Deployment

### Deployment Strategy

The system uses a blue-green deployment strategy for zero-downtime deployments:

1. **Staging Environment**: Full staging environment for testing
2. **Blue-Green Deployment**: Zero-downtime production deployments
3. **Canary Releases**: Gradual rollout of new features
4. **Rollback Capability**: Quick rollback on issues

### Production Requirements

- **Kubernetes Cluster**: 3+ node cluster with proper resource allocation
- **Load Balancer**: For traffic distribution
- **Monitoring**: Full observability stack
- **Backup**: Regular database and configuration backups
- **Security**: Network policies and security scanning

### Deployment Commands

```bash
# Deploy to staging
kubectl apply -f k8s/staging/
kubectl rollout status deployment/recommendation-system-staging

# Deploy to production (blue-green)
kubectl apply -f k8s/production/blue/
kubectl wait --for=condition=ready pod -l app=recommendation-system-blue
kubectl patch service recommendation-system-service -p '{"spec":{"selector":{"app":"recommendation-system-blue"}}}'

# Deploy green version
kubectl apply -f k8s/production/green/
kubectl wait --for=condition=ready pod -l app=recommendation-system-green
kubectl patch service recommendation-system-service -p '{"spec":{"selector":{"app":"recommendation-system-green"}}}'
```

## 🤝 Contributing

### Contribution Guidelines

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/your-feature`
3. **Make changes and commit**: `git commit -m 'Add your feature'`
4. **Push to the branch**: `git push origin feature/your-feature`
5. **Create a Pull Request**

### Code Style

- Follow Rust, Python, and JavaScript community standards
- Use consistent formatting with `cargo fmt`, `ruff`, and `prettier`
- Write clear, descriptive commit messages
- Include tests for new features

### Development Standards

- **Code Review**: All changes require code review
- **Testing**: All features must have appropriate tests
- **Documentation**: Update documentation for new features
- **Security**: Follow security best practices

## 📄 License

This project is licensed under the MIT License - see the LICENSE(LICENSE) file for details.

## 📞 Support

For support and questions:

- **Issues**: [GitHub Issues](https://gitlab.com/Bongas_Squad/bongas-ai/issues)
- **Discussions**: [GitHub Discussions](https://gitlab.com/Bongas_Squad/bongas-ai/discussions)
- **Documentation**: Implementation Guide

## 🙏 Acknowledgments

This project was developed with contributions from the open-source community. Special thanks to:

- The Rust community for excellent tooling and documentation
- The Python ML community for powerful libraries and frameworks
- Kubernetes community for container orchestration

---

**Note**: This is a comprehensive documentation repository. For actual implementation, refer to the specific phase documentation and implementation guide.
