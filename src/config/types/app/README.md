# 🏗️ Config Type: App

The root configuration structure. `AppConfig` is the single source of truth for the entire application, aggregating all domain-specific settings into a unified, immutable object.

---

## 🏗️ Aggregation Model

```mermaid
graph TD
    App[AppConfig] --> S[Server]
    App --> D[Database]
    App --> R[Redis]
    App --> C[ClickHouse]
    App --> I[Ingestion]
    App --> M[ML]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#️-config-type-app)
