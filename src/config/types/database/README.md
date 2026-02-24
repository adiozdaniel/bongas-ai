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
[⬅️ Back to Types Main](../README.md)
