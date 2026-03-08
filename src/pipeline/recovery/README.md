# 📥 Pipeline: RECOVERY (The Stage)

> **The retrieval domain — gathering candidate items from high-performance sources.**

Recovery stages represent the "Stage" of the pipeline, where initial data is brought into the engine. These components are optimized for high-concurrency and resilient I/O, utilizing ClickHouse, Postgres, and Redis to fetch the raw material for our recommendation models.

[🏠 Hub](../../../docs/HUB.md) | [🏗️ Architecture](../../../docs/architecture/SYMPHONY.md) | [📖 Pipeline Main](../README.md)

---

## 🏛️ Recovery Principles

- **Candidate Retrieval**: Fetches a broad set of items based on simple heuristic or collaborative signals.
- **I/O Resilience**: Every stage is protected by a dedicated circuit breaker to prevent slow data sources from degrading the entire cluster.
- **Asynchronous Execution**: Stages are designed to execute in parallel, allowing the engine to pull from multiple sources concurrently.

---

## 🧩 Retrieval Manifest

| Stage | Description | Source |
| :--- | :--- | :--- |
| [**👤 User Preferences**](./fetch_user_preferences/README.md) | Fetches items based on explicit user profile interests. | Postgres |
| [**🔍 Similar Content**](./fetch_similar_content/README.md) | Vector-search based retrieval for "More like this". | Postgres |
| [**🆕 New Releases**](./fetch_new_releases/README.md) | Time-based retrieval for recently published items. | Postgres |
| [**📈 Popular Content**](./fetch_popular_content/README.md) | Global trending items from Postgres or ClickHouse. | Postgres |
| [**📺 Because You Watched**](./fetch_because_you_watched/README.md) | Collaborative filtering based on user interaction. | Postgres |
| [**📂 By Category**](./fetch_by_category/README.md) | Targeted retrieval for specific content taxonomies. | Postgres |
| [**🏗️ ClickHouse Trending**](./fetch_clickhouse_trending/README.md) | High-volume analytical retrieval. | ClickHouse |
| [**⏳ Watch Progress**](./fetch_clickhouse_watch_progress/README.md) | Resumes items the user has partially consumed. | ClickHouse |
| [**🗓️ Seasonal**](./fetch_seasonal_content/README.md) | Context-aware retrieval for specific events. | Postgres |
| [**⏪ Recent Watches**](./fetch_recent_watches/README.md) | Retrieves the user's most recent activity history. | Postgres |

---

[🏠 Hub](../../../docs/HUB.md) | [🔝 Top](#-pipeline-recovery-the-stage)
