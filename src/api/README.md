# 🔌 API: The Symphony Gateway

The API layer is the orchestrator of the Bongas-AI user experience. It follows a **Context-Aware Infrastructure** model, strictly separating high-throughput discovery streams from administrative governance.

[🏠 Hub](../../docs/HUB.md) | [🏗️ Architecture](../../docs/architecture/SYMPHONY.md) | [📖 API Reference](../../docs/api/REFERENCE.md)

---

## 🏛️ Architectural Pillars

The API is structured into functional domains based on the **Audience-Based Pillar** pattern:

### 1. [🌍 THE STAGE (Discovery)](./v1/stage/README.md)

The public gateway for client applications. Optimized for **Zero Perceived Latency** through parallel SSE streaming and frictionless ingestion.

### 2. [🔐 THE BACKSTAGE (Admin)](./v1/backstage/README.md)

The administrative command center. Handles layout compositions, ML strategy management, and **Atomic Engine Reloads**. Protected by strict System Key authorization.

### 3. [💓 THE PULSE (Operations)](./v1/pulse/README.md)

The infrastructure observability layer. Provides health probes, circuit breaker metrics, and real-time cache performance stats.

---

## 🏎️ Context-Aware Infrastructure

Bongas-AI treats different types of traffic with specialized logic at the networking layer:

- **Streaming (SSE)**: Bypasses global JSON compression to ensure immediate bit-delivery. Uses **Parallel Pipelining** to execute up to 5 scenarios concurrently.
- **Feedback (Ingest)**: Asynchronous, fire-and-forget ingestion that enriches user data with device fingerprints in real-time.
- **CRUD (Management)**: Uses standard JSON compression and synchronous validation for high-integrity configuration updates.

---

## 🧩 Module Structure

| Module | Description |
| :--- | :--- |
| [**🛰️ V1**](./v1/README.md) | Versioned functional pillars (Stage, Backstage, Pulse). |
| [**🛡️ Middleware**](./middleware/README.md) | Identity extraction, adaptive rate limiting, and resilience wrappers. |
| [**🧬 Models**](./models/README.md) | Standardized Symphony context and response envelopes. |
| [**🎼 Router**](./router/README.md) | The single source of truth for API assembly and composition. |

---

[🏠 Hub](../../docs/HUB.md) | [🔝 Top](#-api-the-symphony-gateway)
