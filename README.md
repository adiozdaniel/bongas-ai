# BONGAS-AI

High-performance recommendation system serving infrastructure built in Rust.

## 🚀 Features

- **Ultra-fast serving**: Sub-100ms response times with async Rust
- **ONNX inference**: Native ONNX Runtime integration for ML models
- **Multi-algorithm support**: Collaborative filtering, content-based, hybrid approaches
- **Real-time recommendations**: Dynamic scenario-based recommendation pipelines
- **A/B testing**: Built-in experimentation framework with bandit algorithms
- **Production ready**: Comprehensive monitoring, caching, and security

## 📦 Installation

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Redis 7+
- Docker 20+ (for development)

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd bongas-ai

# Install dependencies
cargo install cargo-audit cargo-deny

# Build the project
cargo build --release

# Run migrations
cargo run --bin migrate

# Start the server
cargo run
```

## 🏗️ Architecture

BONGAS-AI is the Rust-based serving infrastructure that provides:

- **High-performance API**: Async Rust backend with comprehensive middleware
- **ONNX model serving**: Native ONNX Runtime for ML inference without Python
- **Dynamic scenarios**: JSONB-based pipeline system for flexible recommendation logic
- **Multi-level caching**: L1 (Redis) and L2 (PostgreSQL) caching with smart invalidation
- **Real-time analytics**: ClickHouse integration for real-time metrics
- **Security**: 8-layer security system with license validation and anti-debugging

### Project Structure

```text
bongas-ai/
├── src/
│   ├── api/           # REST API endpoints
│   ├── bongas/        # Core BONGAS engine
│   ├── scenarios/     # Dynamic scenario system
│   ├── ml/           # ONNX-based ML infrastructure
│   ├── experiments/  # A/B testing & bandits
│   ├── cache/        # Multi-level caching
│   ├── kafka/        # Event consumers
│   ├── analytics/    # Real-time metrics
│   └── security/     # Security layer
├── models/           # ONNX model files
├── config/           # Configuration files
├── migrations/       # Database migrations
├── scripts/          # Build & deployment scripts
└── docs/            # Documentation
```

## 📖 Documentation

- [Architecture](docs/guides/architecture.md) - System design and data flow
- [Scenarios](docs/guides/scenarios.md) - Dynamic pipeline configuration
- [ML Integration](docs/guides/ml_integration.md) - ONNX model serving
- [ONNX Deployment](docs/guides/onnx_deployment.md) - Model lifecycle management
- [API Reference](docs/guides/api_reference.md) - REST endpoints documentation

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test integration

# Run performance tests
cargo test --release --features=performance
```

## 📊 Monitoring

The system includes comprehensive monitoring:

- **Prometheus metrics**: Application and business metrics
- **Grafana dashboards**: Visualization and alerting
- **Real-time analytics**: ClickHouse integration
- **Performance benchmarks**: Built-in benchmarking suite

## 🔒 Security

- JWT-based authentication
- Role-based access control
- Input validation and sanitization
- Rate limiting and DDoS protection
- 8-layer security system with license validation

## 🚀 Deployment

### Development

```bash
# Start with Docker Compose
docker-compose up -d

# Run in development mode
cargo run
```

### Production

```bash
# Build release binary
cargo build --release

# Package for deployment
./scripts/package.sh

# Deploy with Kubernetes
kubectl apply -f k8s/
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make changes and commit: `git commit -m 'Add your feature'`
4. Push to the branch: `git push origin feature/your-feature`
5. Create a Pull Request

## 📄 License

MIT License - see [LICENSE](LICENSE) file for details.

## 📞 Support

- [Issues](https://gitlab.com/Bongas_Squad/bongas-ai/issues)
- [Discussions](https://gitlab.com/Bongas_Squad/bongas-ai/discussions)

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
