# 🖱️ Database Repository: Interaction

> **High-throughput storage for user interaction events.**

This repository is the primary sink for the ingestion pipeline. it records every click, impression, and playback session, providing the raw data needed for both real-time personalization and offline model training.

---

## 🏗️ Ingestion Flow

```mermaid
graph LR
    Pipe[Ingestion Pipeline] -->|Normalize| Activity[UserActivity]
    Activity -->|Upsert| Repo[Interaction Repository]
    Repo -->|Store| DB[(user_interactions)]
```

---

## 📄 Interaction Payloads

To ensure architectural decoupling, this repository uses specialized DTOs for recording data:

- **`InteractionPayload`**: Unified payload for single events (clicks, impressions, reactions).
- **`BatchInteractionPayload`**: Optimized structure for high-volume batch inserts.

---

[🏠 Hub](../../../../docs/HUB.md) | [📦 Back to Database Main](../README.md) | [🔝 Top](#️-database-repository-interaction)
