# 🌍 THE STAGE: Public Discovery Gateway

The Stage is the primary initiation point for client applications. It is optimized for high-throughput content delivery and real-time behavioral feedback.

[🏠 Hub](../../../../docs/HUB.md) | [🏗️ Architecture](../../../../docs/architecture/SYMPHONY.md) | [⚡ Streaming](../../../../docs/architecture/ORCHESTRATION.md)

---

## 🏗️ Functional Domains

### 🛰️ Discovery (`discovery.rs`)

Responsible for the **Genesis** entry point and **Page Orchestration**.

- **Genesis**: A single root call that resolves the user's personalized Navigation Mesh and initiates the SSE stream for the landing page.
- **Batch-Streaming**: Delivers content rows in server-dictated batches to ensure memory safety on mobile devices.
- **Continuation**: Emits `continuation` events with pre-calculated URLs for seamless deep-scrolling.

### 🧪 Ingestion (`ingestion.rs`)

The frictionless feedback loop for "The Brain."

- **Real-Time Tracking**: Receives clicks, impressions, and watch-time events.
- **Context Enrichment**: Automatically attaches device fingerprints and visitor IDs to incoming events before passing them to the ingestion pipeline.

---

## 🏎️ Performance Standards

| Feature | Target | Technical Strategy |
| :--- | :--- | :--- |
| **Genesis Latency** | < 50ms | Zero-DB resolution via Nav-Mesh caching. |
| **Row Delivery** | Real-time | Parallel Pipelining (Buffered Concurrency: 5). |
| **Ingest Overhead** | Negligible | Fire-and-forget async task spawning. |

---

[🏠 Hub](../../../../docs/HUB.md) | [🔝 Top](#-the-stage-public-discovery-gateway)
