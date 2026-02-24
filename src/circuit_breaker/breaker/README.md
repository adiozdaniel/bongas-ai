# ⚡ Circuit Breaker: Core Execution

The `breaker` sub-module houses the central orchestrator that users interact with. It manages the lifecycle of a single protected operation, coordinating between state, metrics, and observers.

---

## 🛠️ Responsibility Layer

```mermaid
sequenceDiagram
    participant User
    participant Breaker
    participant State
    participant Window
    participant Observer

    User->>Breaker: call(operation)
    Breaker->>State: check_permission()
    State-->>Breaker: OK / Rejected
    
    alt is OK
        Breaker->>User: execute(operation)
        User-->>Breaker: Result (Success/Failure)
        Breaker->>Window: record_metrics(duration)
        Breaker->>Observer: notify_event()
    else is Rejected
        Breaker-->>User: CircuitBreakerError::Rejected
    end
```

---

## 🔑 Key Features

- **Thundering Herd Protection**: Integration with request consolidation.
- **Bulkhead Pattern**: Built-in semaphore-based concurrency limiting.
- **Async First**: Designed for native `tokio` integration.
- **Fast Path**: Optimized state checks with minimal lock contention.

---
[⬅️ Back to Circuit Breaker Main](../README.md)
