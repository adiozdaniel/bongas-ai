# 📊 Database: Metrics

Provides deep visibility into database performance, including query latencies, pool utilization, and error rates per repository.

---

## 🏗️ Monitoring Architecture

```mermaid
graph LR
    Pool[Resilient Pool] -->|Latency/Status| Metrics[Database Metrics]
    Repo[Repositories] -->|Query Type| Metrics
    Metrics -->|Aggregate| Snapshot[Metrics Snapshot]
```

---
[⬅️ Back to Database Main](../README.md)
