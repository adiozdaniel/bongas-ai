# 🎼 Router: The Grand Composer

The Router is the single source of truth for the Bongas-AI API. It orchestrates the Stage, Backstage, and Pulse pillars into a unified hierarchical discovery graph.

[🏠 Hub](../../../../docs/HUB.md) | [🏗️ Architecture](../../../../docs/architecture/SYMPHONY.md) | [📖 API Reference](../../../../docs/api/REFERENCE.md)

---

## 🏗️ Architecture: Context-Aware Routing

Bongas-AI implements a **Context-Aware Routing** engine that optimizes infrastructure logic based on the functional pillar:

### 1. 🌍 The Stage (Streaming)

- **Genesis & Orchestration**: Optimized for raw SSE bit-delivery. **Selective Compression** is disabled for these routes to prevent buffering latency.
- **Data & Ingestion**: JSON-heavy routes use Gzip/Zstd compression to minimize payload size.

### 2. 🔐 The Backstage (Governance)

- **Pillar-Level Security**: A unified System Key authorization shield is applied at the root of the Backstage pillar.
- **Administrative Bulkhead**: Isolated from the public discovery hot-path to prevent management tasks from impacting user latency.

### 3. 💓 The Pulse (Pulse)

- **Ops Probes**: Fast, uncompressed health and readiness checks for container orchestrators.

---

## 🧩 Assembly Logic

The router is composed in `service.rs`, following a strictly layered approach:

```rust
// 1. Build Pillars
let stage = stage::discovery::routes();
let backstage = backstage::orchestration::routes();
let pulse = pulse::health::routes();

// 2. Compose Symphony
let symphony = stage.nest("/admin", backstage);

// 3. Mount Registry
Router::new().nest("/recommendation", symphony).merge(pulse)
```

---

[🏠 Hub](../../../../docs/HUB.md) | [📡  Back to API Main](../README.md) | [🔝 Top](#-router-the-grand-composer)
