# ⚙️ Ingestion: Processor

The worker component that consumes raw `UserActivity` events from the internal channel. It performs normalization, database persistence, and triggers downstream cache invalidation via the Staleness Engine.

---

## 🛠️ Processing Flow

```mermaid
sequenceDiagram
    participant Source
    participant Processor
    participant DB
    participant Staleness
    
    Source->>Processor: Push UserActivity
    Processor->>Processor: Normalize Data
    par Save to DB
        Processor->>DB: UPSERT user_interactions
    and Invalidate Cache
        Processor->>Staleness: notify_event()
    end
```

---
[⬅️ Back to Ingestion Main](../README.md)
