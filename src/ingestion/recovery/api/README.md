# ⚡ Ingestion Source: API

Provides a direct endpoint for the application or front-end clients to push user activities (like clicks, impressions, or conversions) directly into the recommendation engine.

---

## 🛠️ Data Flow

```mermaid
sequenceDiagram
    participant Client
    participant API as API Source
    participant Chan as Internal Channel
    
    Client->>API: POST /api/v1/recommendations/ingest
    API->>API: Validate Payload
    API->>Chan: Send UserActivity
    API-->>Client: 202 Accepted
```

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Recovery Main](../README.md)
