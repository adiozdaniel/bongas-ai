# ⚙️ Search: Manager

> **The atomic lifecycle controller for the search index.**

The Manager provides thread-safe access to the embedded search index. It ensures that all updates are committed atomically and that the search reader stays synchronized with the writer.

## 🛡️ Resilience Patterns

- **Atomic Commits:** Uses Tantivy's ACID-compliant commit logic to prevent index corruption.
- **Auto-Repair:** Background integrity checks that can trigger a full re-index if the local state is lost.
- **Bulkhead:** Dedicated memory budget for indexing to prevent interference with recommendation logic.

---

[🏠 Hub](../../../docs/HUB.md) | [⬅️ Back to Search](../README.md)
