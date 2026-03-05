# 🌐 Domain Errors: Functional Taxonomy

> **The specialized error types for Bongas-AI subsystems.**

Domain errors are grouped into functional categories. Each category implements the `ErrorClassifier` trait to ensure compatibility with the global resilience system.

---

## 📂 Sub-Taxonomies

| Category | Description |
| :--- | :--- |
| **Database** | Errors for Redis, Postgres, and ClickHouse. |
| **Engine** | Pipeline, Model Inference, and Scenario orchestration failures. |
| **Infrastructure** | Cache, Ingestion, Middleware, and Metrics. |
| **Security** | License, Binary integrity, and Hardware validation. |

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Error Main Documentation](../README.md)
