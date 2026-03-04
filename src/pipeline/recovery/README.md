# 📥 Pipeline Stages: Fetch

> **Retrieval components for gathering candidate items.**

Fetch stages are usually the entry point of a pipeline or a branch. They interface with databases, caches, and ML services to retrieve an initial set of items.

---

## 🧩 Sub-Modules

| Stage | Description |
| :--- | :--- |
| [**👤 User Preferences**](./fetch_user_preferences/README.md) | Fetches items based on explicit user profile interests. |
| [**🔍 Similar Content**](./fetch_similar_content/README.md) | Vector-search based retrieval for "More like this". |
| [**🆕 New Releases**](./fetch_new_releases/README.md) | Time-based retrieval for recently published items. |
| [**📈 Popular Content**](./fetch_popular_content/README.md) | Global trending items from ClickHouse or Redis. |
| [**📺 Because You Watched**](./fetch_because_you_watched/README.md) | Collaborative filtering based on the user's last interaction. |
| [**📂 By Category**](./fetch_by_category/README.md) | Targeted retrieval for specific content taxonomies. |
| [**🏗️ ClickHouse Trending**](./fetch_clickhouse_trending/README.md) | High-volume analytical retrieval from ClickHouse. |
| [**⏳ Watch Progress**](./fetch_clickhouse_watch_progress/README.md) | Resumes items the user has partially consumed. |
| [**🗓️ Seasonal**](./fetch_seasonal_content/README.md) | Context-aware retrieval for specific dates/events. |
| [**⏪ Recent Watches**](./fetch_recent_watches/README.md) | Retrieves the user's most recent activity history. |

---
[⬅️ Back to Stages Main](../README.md)
