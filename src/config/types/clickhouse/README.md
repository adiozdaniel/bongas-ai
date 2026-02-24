# 📊 Config Type: ClickHouse

Settings for the ClickHouse OLAP database integration. Used for high-volume analytics, long-term interaction storage, and performance monitoring.

---

## 🏗️ Configuration Surface

```mermaid
graph LR
    CH[ClickHouseConfig] --> Conn[Connection Pool]
    CH --> Auth[Credentials]
    CH --> Opts[Timeouts / Retries]
```

---
[⬅️ Back to Types Main](../README.md)
