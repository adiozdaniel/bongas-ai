# 💤 Fatigue Sync: State Synchronization Worker

The Fatigue Sync worker is responsible for tracking and synchronizing item exposure state across the Bongas-AI environment. This state is used by the `FilterContentFatigueStage` to prevent user over-exposure to the same items.

---

## 🏛️ Adaptor Architecture

The worker uses a pluggable adaptor pattern to support different deployment environments:

| Adaptor | Type | Description |
| :--- | :--- | :--- |
| **InternalHook** | Request-Path | Directly increments Redis counters during the API response flow. Ideal for standalone deployments. |
| **KafkaStream** | Event-Driven | Consumes exposure and engagement events from Kafka topics. Designed for high-scale distributed environments. |
| **ClickHousePoll** | Analytical | Periodically polls ClickHouse for recent exposures and engagement to reconcile state. |

---

## 🚀 Key Functions

- **Exposure Tracking**: Increments a Redis counter `seen:{profile_id}:{item_id}` for every item recommended to a user.
- **Engagement Reset**: Deletes the fatigue counter when a user engages with an item (click, playback, reaction), making it eligible for full scoring again.
- **TTL Management**: Automatically expires fatigue counters after 7 days to ensure state doesn't grow indefinitely.

---

## ⚙️ Configuration

Configured via `ml` section in `AppConfig`:

```toml
[ml]
fatigue_enabled = true
fatigue_adaptor = "internal_hook"
fatigue_max_exposures = 5
fatigue_penalty_factor = 0.8
```

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-fatigue-sync-state-synchronization-worker)
