# 💓 THE PULSE: Operations & Observability

The Pulse provides real-time visibility into the health and performance of the Bongas-AI Symphony. It is the primary interface for infrastructure monitoring and automated orchestration tools.

[🏠 Hub](../../../../docs/HUB.md) | [🏗️ Architecture](../../../../docs/architecture/SYMPHONY.md) | [🔒 Security](../../../../docs/architecture/SECURITY.md)

---

## 🏗️ Functional Domains

### 🏥 Health (`health.rs`)

Standardized probes for container orchestration (Kubernetes).

- **Liveness**: Confirms the process is running.
- **Readiness**: Confirms all external dependencies (Postgres, Redis, ClickHouse) are reachable and the engine is hydrated.

### 📊 Metrics (`metrics.rs`)

Deep observability into the engine's resilience and throughput.

- **Registry Snapshot**: Real-time state of all circuit breakers and bulkheads.
- **Cache Observability**: L1/L2 hit ratios and staleness invalidation stats.
- **Ingestion Health**: Monitoring of Kafka consumer lag and ClickHouse backfill velocity.

---

## 🛡️ Resilience Standards

The Pulse is powered by the **Resilience Middleware**, which ensures that monitoring traffic never overwhelms the system. All metrics are aggregated using atomic counters to ensure zero performance impact on the hot path.

---

[🏠 Hub](../../../../docs/HUB.md) | [📡  Back to API Main](../README.md) | [🔝 Top](#-the-pulse-operations--observability)
