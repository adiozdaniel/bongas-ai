# 💎 API V1: Features

Provides introspection into the Feature Store. Enables developers and ML engineers to query available features and their current values for specific items or users.

---

## 🏗️ Feature Query Flow

```mermaid
graph TD
    User[ML Engineer] -->|GET /features| API[Features API]
    API -->|Query| Store[Feature Store]
    Store -->|Lookaside| Cache[Redis L1]
    Store -->|Primary| DB[(PostgreSQL)]
    API --> Result[Feature Values]
```

---
[⬅️ Back to V1 Main](../README.md)
