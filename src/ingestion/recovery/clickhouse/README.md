# 🛠️ Ingestion Source: ClickHouse

A polling-based source that periodically queries ClickHouse tables to synchronize historical or batch-processed interactions. Useful for integrating with existing OLAP data pipelines.

---

## 🏗️ Polling Cycle

```mermaid
graph LR
    Timer[Interval] --> Poll[Query ClickHouse]
    Poll --> Batch[Result Set]
    Batch -->|Iterate| activity[UserActivity]
    activity --> Chan[Internal Channel]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Recovery Main](../README.md)
