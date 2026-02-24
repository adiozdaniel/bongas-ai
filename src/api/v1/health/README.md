# 🏥 API V1: Health

Endpoints for load balancer probes and deep system health checks. Provides a quick way to verify that the service and all its dependencies are operational.

---

## 🏗️ Health Check Flow

```mermaid
graph TD
    LB[Load Balancer] -->|GET /health| API[Health API]
    API -->|Ping| DB[(PostgreSQL)]
    API -->|Ping| Cache[Redis]
    API -->|Pulse| Ingest[Ingestion]
    API -->|200 OK| LB
```

---
[⬅️ Back to V1 Main](../README.md)
