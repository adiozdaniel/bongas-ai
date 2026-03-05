# ⚡ Boost: Affinity Freshness

Intelligent discovery stage that boosts new content only if it aligns with specific user affinities (e.g., genre, language, tribe). This prevents "discovery fatigue" by showing new items that are actually relevant.

---

## ⚙️ Parameters

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `max_boost` | float | 5.0 | Maximum multiplier for a perfect match. |
| `freshness_window_hours` | int | 48 | Age limit for items to be eligible for boost. |
| `affinity_key` | string | "genre" | Metadata key used for affinity matching. |

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Boost Category](../README.md)
