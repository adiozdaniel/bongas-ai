# 🖼️ Pages: Smart SDUI Orchestration

> **The Dynamic Blueprint for the Bongas-AI Symphony.**

The Pages module is the heart of the **Server-Driven UI (SDUI)** engine. It transforms raw recommendation scenarios into high-fidelity UI compositions, resolving the best layout for every user's device and context.

[🏠 Hub](../../../../docs/HUB.md) | [🏗️ Architecture](../../../../docs/architecture/SYMPHONY.md) | [🎨 Smart Pages](../../../../docs/architecture/PAGES.md)

---

## 🏗️ Core Capabilities

- **Contextual Resolver**: Hierarchically resolves layouts based on `device_type`, `maturity_rating`, and `priority`.
- **Structured Composition**: Dictates not just _what_ content to show, but _how_ to show it via `row_type` and `row_style` metadata.
- **Dynamic Canvas**: Admins can reorder, swap, or experiment with page structures in real-time from the database.
- **Resilient Caching**: Employs a multi-tenant LRU cache keyed by (Slug, Device, Maturity) for sub-millisecond resolution.

---

## 🧩 Module Structure

| Module                                | Description                                                            |
| :------------------------------------ | :--------------------------------------------------------------------- |
| [**🏷️ Types**](./types/README.md)     | Enriched SDUI models including `PageCompositionItem` and `PageLayout`. |
| [**🕹️ Manager**](./manager/README.md) | The resolution engine and caching logic for page blueprints.           |

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#️-pages-smart-sdui-orchestration)
