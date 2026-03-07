# 🏢 Cache: Manager

The central orchestrator of the caching system. It provides a unified API for the rest of the application, hiding the complexity of 3-tier lookups and backfilling.

---

## 🛠️ Execution Flow

```mermaid
sequenceDiagram
    participant App
    participant Manager
    participant L1 (LRU)
    participant L2 (Redis)
    participant L3 (Postgres)
    
    App->>Manager: get(key)
    Manager->>L1: check()
    alt L1 Hit
        L1-->>Manager: Result
    else L1 Miss
        Manager->>L2: check()
        alt L2 Hit
            L2-->>Manager: Result
            Manager->>L1: backfill()
        else L2 Miss
            Manager->>L3: check()
            alt L3 Hit
                L3-->>Manager: Result
                Manager->>L2: backfill()
                Manager->>L1: backfill()
            else L3 Miss
                Manager-->>App: None
            end
        end
    end
    Manager-->>App: Result
```

---

[🏠 Hub](../../../docs/HUB.md) | [🗄️ Back to Cache Main](../README.md) | [🔝 Top](#-cache-manager)
