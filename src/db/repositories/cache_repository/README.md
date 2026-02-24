# 🗄️ Database Repository: Cache

> **Persistence for long-term cache metadata and pre-warming schedules.**

While the actual cached items live in Redis (L2), the `cache_repository` manages the persistent metadata, warming rules, and hydration schedules that drive the `PredictiveWarmer`.

---

## 🏗️ Hydration Logic

```mermaid
graph TD
    Timer[Warming Schedule] -->|Query| Repo[Cache Repository]
    Repo -->|Rules| Warmer[Cache Warmer]
    Warmer -->|Hydrate| Redis[(Redis L2)]
```

---
[⬅️ Back to Repositories Main](../README.md)
