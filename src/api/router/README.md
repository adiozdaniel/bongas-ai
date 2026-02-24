# 🚀 API: Router

The `router` sub-module is the final assembly point for the API. It coordinates the nesting of versioned routes and the application of global middleware chains.

---

## 🏗️ Router Assembly

```mermaid
graph LR
    Assembly[Router::new()] --> V1[Nest /api/v1]
    V1 --> Health[Nest /health]
    Health --> MW[Apply Middleware]
    MW --> Final[Ready for Serving]
```

---
[⬅️ Back to API Main](../README.md)
