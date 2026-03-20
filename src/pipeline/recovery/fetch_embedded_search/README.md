# 🔍 Fetch: Embedded Search

Performs localized, zero-latency keyword search against the embedded Tantivy index. This stage executes directly within the binary memory space, avoiding network overhead.

## 🌟 Key Features

- **Relevance Fusion:** Hybrid ranking merging BM25 keyword scores with Vision DNA vector similarity.
- **Sheng-Native:** Utilizes specialized tokenizers for Swahili/Sheng matching.
- **Zero-Network:** No external service calls; results are retrieved from memory-mapped disk files.

---

[🏠 Hub](../../../../docs/HUB.md) | [⬅️ Back to Fetch Category](../README.md)
