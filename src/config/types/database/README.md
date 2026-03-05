# 🗄️ Config Type: Database

PostgreSQL connection settings. Manages connection pooling, read replicas, and transaction timeouts for the relational storage layer.

---

## 🏗️ Persistence Layer

```mermaid
graph LR
    DB[DatabaseConfig] --> Pool[Connection Pool]
    DB --> Reps[Read Replicas]
    DB --> Logic[Timeout Policies]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🧬 Back to Types Main](../README.md) | [🔝 Top](#️-config-type-database)
