# 🔄 Search Sync Worker: The Search Pulse

The Search Sync Worker is a background intelligence task responsible for maintaining a real-time, high-fidelity mirror of the PostgreSQL content catalog within the Meilisearch index.

---

## 🚀 Lifecycle

1. **Setup**: On startup, it ensures the Meilisearch index exists and is configured with optimal ranking rules (proximity, typo-tolerance, popularity-desc).
2. **Scan**: Periodically fetches active items from the PostgreSQL `item_features` table, ordered by `updated_at`.
3. **Map**: Transforms complex database rows into optimized `SearchDocument` DTOs (flattening genres, tags, and creators).
4. **Sync**: Batches and pushes documents to Meilisearch using the asynchronous `add_documents` API.

---

## 🎯 Strategic Impact

- **Zero-Lag Discovery**: Ensures that new or updated content is searchable within seconds of entering the catalog.
- **Off-Main-Path Efficiency**: Handles indexing on the **Intelligence Plane**, preventing write-heavy search updates from impacting the performance of the Stage (Discovery Path).
- **Typo Tolerance**: Leverages Meilisearch's Rust-native engine to provide instant, error-resilient results for user queries.

---

[🏠 Hub](../../../../../docs/HUB.md) | [⬅️ Back to Workers Main](../README.md)
