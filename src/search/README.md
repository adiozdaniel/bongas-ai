# 🔍 Search Pillar: Sovereign Embedded Search

> **High-performance, zero-dependency search built into the core binary.**

The Search Pillar replaces external search engines (like Meilisearch) with an embedded **Tantivy** index. This ensures 100% data sovereignty, zero network latency, and a minimal memory footprint.

## 🏗️ Architecture

- **[Manager](./manager)**: Handles index lifecycle, atomic commits, and auto-repair logic.
- **[Schema](./schema)**: Multi-modal definition supporting Keywords, Spoken Content, and Vision DNA.
- **[Analyzer](./analyzer)**: Sheng-native linguistic analysis using `jieba-rs`.

## 🚀 Key Features

1. **Zero-Latency:** Search results are retrieved directly from memory-mapped disk files without network overhead.
2. **Deep Content:** Spoken words are extracted from videos and indexed for "heard-phrase" searching.
3. **Relevance Fusion:** Hybrid ranking combining BM25 keyword scores with Vision DNA vector similarity.
4. **Indestructible:** Built-in self-healing that automatically rebuilds the index if files are corrupted or missing.

---

[🏠 Hub](../../docs/HUB.md) | [🏗️ Architecture](../../docs/architecture/SYMPHONY.md)
