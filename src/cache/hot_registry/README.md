# 🔥 Cache: Hot Registry

A highly-optimized, read-heavy registry for the application's most popular content. It acts as a "Fast Path" for items that bypass standard LRU/Redis lookups due to their extreme popularity.

---

## 🏗️ Design Pattern

```mermaid
graph LR
    Engine[Bongas Engine] -->|Popularity Signal| Registry[Hot Registry]
    App[Request] -->|High-Speed Check| Registry
    Registry -->|Hit| Fast[Instant Return]
    Registry -->|Miss| Standard[Standard Cache Path]
```

---
[⬅️ Back to Cache Main](../README.md)
