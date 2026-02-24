# 🛠️ Cache: Strategies

Concrete implementations of the `CacheStrategy` trait. Supports different storage backends with pluggable logic.

---

## 🧩 Implementations

| Strategy | Description |
| :--- | :--- |
| **LRU** | Fast, local memory storage with least-recently-used eviction. |
| **Redis** | Distributed storage for cross-instance consistency. |
| **NoOp** | Transparent pass-through for testing or disabling cache. |

---
[⬅️ Back to Cache Main](../README.md)
