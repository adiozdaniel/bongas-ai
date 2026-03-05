# 🎭 Pipeline: PROCESSING (The Backstage)

> **The transformation domain — constraint enforcement, data hydration, and deduplication.**

Processing stages represent the "Backstage" of the pipeline, where raw candidates are filtered, enriched, and normalized. This domain is responsible for the system's "Safety and Quality" filters, ensuring that recommendations are legal, available, and deduplicated before the final intelligence layer.

[🏠 Hub](../../../docs/HUB.md) | [🏗️ Architecture](../../../docs/architecture/SYMPHONY.md) | [📖 Pipeline Main](../README.md)

---

## 🏛️ Processing Principles

- **Safety & Compliance**: All age-ratings, regional availability, and explicit content filters live in this pillar.
- **Data Hydration**: Enriches candidate items with dynamic metadata like "Time Remaining" or "Watching Now" counters.
- **Uniqueness**: Deduplication ensures a clean, unique result set across parallel retrieval branches.

---

## 🧩 Transformation Manifest

| Stage | Domain | Description |
| :--- | :--- | :--- |
| [**👤 Maturity Filter**](./maturity_filter/README.md) | Safety | Enforces age-appropriate content for the current user. |
| [**🔞 Explicit Content**](./filter_explicit_content/README.md) | Safety | Removes adult or restricted content from the result set. |
| [**🚫 Already Watched**](./filter_already_watched/README.md) | Logic | Filters out items the user has already consumed. |
| [**🌍 By Country**](./filter_by_country/README.md) | Availability | Enforces regional licensing restrictions. |
| [**📉 By Rating**](./filter_by_rating/README.md) | Quality | Removes low-quality or poorly rated content. |
| [**⏳ Time Remaining**](./enrich_time_remaining/README.md) | Hydration | Adds dynamic watch-progress metadata to items. |
| [**✂️ Deduplicate**](./deduplicate/README.md) | Normalization | Ensures item IDs are unique in the result set. |
| [**🎯 By Language**](./filter_by_language/README.md) | Relevance | Filters items by the user's preferred audio/subtitle languages. |
| [**💳 Subscription**](./filter_by_subscription_tier/README.md) | Logic | Enforces content access based on user tier. |

---

[🏠 Hub](../../../docs/HUB.md) | [🔝 Top](#-pipeline-processing-the-backstage)
