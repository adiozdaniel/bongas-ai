# 💓 Engine: Workers Manager

Coordinates the system's "heartbeat" tasks. This includes periodic cache warming, Hot Registry updates based on trending scores, and ClickHouse performance pruning for low-performing scenarios.

---

## 📅 Background Schedule

| Task | Interval | Duty |
| :--- | :--- | :--- |
| **Hot Registry** | 1 Minute | Refresh top 100 trending items. |
| **Cache Warmer** | Configurable | Proactively hydrate L2 for key scenarios. |
| **CTR Pruning** | On Demand | Identify scenarios with < 1% click-through. |

---
[⬅️ Back to Engine Main](../README.md)
