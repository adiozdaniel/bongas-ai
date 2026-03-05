# 📊 Ingestion: Metrics

Monitors the health and performance of all ingestion sources and the processor. Provides aggregated throughput stats and source-level status reporting.

---

## 🏗️ Monitoring Model

```mermaid
graph TD
    Sources[All Sources] -->|Pulse| Metrics[Ingestion Metrics]
    Processor[Processor] -->|Success/Fail| Metrics
    Metrics -->|Aggregate| Health[Health Summary]
    Health --> Dashboard[Admin API / Grafana]
```

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Broadcast Main](../README.md)
