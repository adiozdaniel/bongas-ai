# 🗄️ Config Type: Redis

Settings for the L2 distributed cache and rate limiting layer. Controls connection pooling, cluster mode, and request-level timeouts.

---

## 🏗️ Redis Integration

```mermaid
graph LR
    Red[RedisConfig] --> Conn[Cluster / Single Node]
    Red --> Pool[Pool Settings]
    Red --> TO[Timeouts / Retries]
```

---
[⬅️ Back to Types Main](../README.md)
