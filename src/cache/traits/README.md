# 🧬 Cache: Traits

The foundational abstractions for the caching system. Defines how any storage backend should behave to be compatible with the `CacheManager`.

---

## 🏗️ Interface Definition

```mermaid
classDiagram
    class CacheStrategy {
        <<interface>>
        +get(key) Result
        +set(key, val, ttl) Result
        +delete(key) Result
        +name() String
        +tier() CacheTier
    }
```

---

[🏠 Hub](../../../docs/HUB.md) | [🗄️ Back to Cache Main](../README.md) | [🔝 Top](#-cache-traits)
