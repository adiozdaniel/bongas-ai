# 📥 Analytics: Collector

The `collector` module is the primary entry point for recording system events. It manages an efficient in-memory buffer to aggregate statistics before they are dispatched for upload.

---

## 🛠️ Collection Flow

```mermaid
sequenceDiagram
    participant App
    participant Collector
    participant Buffer
    participant Timer

    App->>Collector: record_event(data)
    Collector->>Collector: filter_and_validate()
    Collector->>Buffer: push(event)
    
    Timer->>Collector: on_interval_reached()
    Collector->>Buffer: drain_to_payload()
    Collector-->>App: (Async Upload Triggered)
```

---

## 🔑 Key Features

- **High Concurrency**: Thread-safe aggregation using atomic operations where possible.
- **Efficient Buffering**: Minimizes lock contention to avoid impacting the application's hot path.
- **Deduplication**: Automatically merges identical rapid-fire events to reduce payload size.

---
[⬅️ Back to Analytics Main](../README.md)
