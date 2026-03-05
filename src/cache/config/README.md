# ⚙️ Cache: Configuration

Defines the behavior and limits of each caching tier, including TTLs, max entries, and background warming schedules.

---

## 🛠️ Data Model

```mermaid
classDiagram
    class CacheConfig {
        +bool l1_enabled
        +usize l1_max_entries
        +Duration l1_ttl
        +bool l2_enabled
        +Duration l2_ttl
        +bool warming_enabled
        +Duration warming_interval
        +Vec warm_scenarios
    }
```

---

[🏠 Hub](../../../docs/HUB.md) | [🗄️ Back to Cache Main](../README.md) | [🔝 Top](#️-cache-configuration)
