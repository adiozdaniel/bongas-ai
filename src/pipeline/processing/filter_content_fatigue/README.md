# 📉 Filter: Content Fatigue

The Content Fatigue stage is a processing component that suppresses or penalizes over-exposed items to prevent user cognitive load and boredom.

---

## 🏎️ Execution Flow

1. **State Fetching**: Uses Redis `MGET` to batch-fetch exposure counts for all candidate items based on the user's `profile_id`.
2. **Hard Filter**: Drops any item that has exceeded the `fatigue_max_exposures` threshold (default: 5).
3. **Soft Penalty**: Applies an exponential penalty to items with existing exposures: `score = score * (penalty_factor ^ count)`.
4. **Audit**: Adds reasoning for penalized items: `Fatigue Penalty: [N] previous exposures`.

---

## 🎯 Design Principles

- **Batch Efficiency**: Fetches all fatigue counters in a single Redis round-trip to maintain sub-millisecond stage latency.
- **Dynamic Adaptability**: The fatigue state is updated in real-time by the `FatigueSynchronizer` and reset immediately upon user engagement.
- **Cold-Start Safe**: Anonymous users or profiles without tracking data are ignored (fail-open).

---

## ⚙️ Configuration

Controlled via `ml` configuration:
- `fatigue_max_exposures`: Maximum allowed impressions before an item is hidden.
- `fatigue_penalty_factor`: The multiplier applied per impression (e.g., 0.8).

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-filter-content-fatigue)
