# 🛠️ API V1: Admin

System-level control and monitoring. Provides deep visibility into the engine's internal state, cache performance, and subsystem health.

---

## 🏗️ Administration Surface

```mermaid
graph TD
    Admin[Platform Admin] -->|Commands| API[Admin API]
    
    subgraph Control Plane
        API --> Cache[Cache Pruning]
        API --> Models[ONNX Reloading]
        API --> Ingest[Ingestion Resets]
    end
    
    subgraph Monitoring
        API --> Metrics[Performance Stats]
        API --> Health[Detailed Health Check]
    end
```

---
[⬅️ Back to V1 Main](../README.md)
