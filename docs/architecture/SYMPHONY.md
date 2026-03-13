# 🏗️ The Bongas-AI Symphony 2.0: Core Architecture

[🏠 Hub](../HUB.md) | [👤 Identity](./IDENTITY.md) | [⚡ Streaming](./ORCHESTRATION.md) | [🎨 Pages](./PAGES.md) | [🔒 Security](./SECURITY.md)

---

## 🎼 The Paradigm: Server-Side Anticipatory Engine

In Symphony 2.0, we have evolved beyond simple Server-Driven UI (SDUI). Bongas-AI is now an **Anticipatory Engine** that minimizes user-perceived latency through server-side look-ahead and parallel fan-out.

**Bongas-AI flips the script.** The server is the Orchestrator, and the client is the Canvas.

```mermaid
graph TD
    A[Genesis Request: /api/v1/recommendation/page/home] --> B[Middleware: OTLP Shield & Identity]
    B --> C{Symphony Resolver}
    C --> D[Identify Context: Visitor/Device/Profile]
    C --> E[Resolve Page Composition & Ranking]
    E --> F[Parallel Execution: Fan-Out factor 5]
    F --> G[Streaming SSE Results]
    F --> H[Ghost Pre-warm: Server-Side Look-Ahead]
    G --> I[Client: Instant-On Rendering]
    H --> J[(Redis Ghost Cache)]
```

## 🌟 Key Architectural Pillars

### 1. Velocity Engine (Parallel Fan-Out)

Symphony 2.0 eliminates sequential bottlenecks. Using Rust's `futures` ecosystem, we execute up to 5 scenarios simultaneously per request.

- **Ordered Pipelining:** Uses `.buffered(5)` for SSE to maintain UI layout integrity.
- **Unordered Ghosting:** Uses `.buffer_unordered(5)` for background pre-warming to maximize throughput.

### 2. The OTLP Shield (High-Performance Observability)

Every request is protected and tracked by a "Shield" of distributed tracing, optimized for high throughput.

- **Trace ID Propagation**: Follows a request from the initial HTTP header, through the SSE fan-out, down to Postgres and Redis.
- **Identity Enrichment**: Spans are automatically enriched with `visitor_id`, `device_hash`, and `profile_id`.
- **Netflix-Scale Batching**: Telemetry is buffered and exported in configurable batches (`batch_size`, `max_queue_size`) to minimize the impact on the engine's performance.
- **Standard Protocol Support**: Supports both gRPC and HTTP OTLP protocols with custom header injection.

### 4. Intelligence & Background Coordination (The Pulse)

The engine's "Intelligence" is not just reactive; it is proactive. Background workers constantly refine the data used by the discovery pipeline.

- **WorkersManager**: Centralized orchestration for all background maintenance tasks and intelligence workers.
- **Behavioral Tribes (TribeOrchestrator)**: Periodically clusters user profiles into behavioral "tribes" using K-Means clustering on embeddings. This enables high-relevance discovery for cold-start users.
- **Hyper-Local Semantic Pulse (RegionalPulseWorker)**: Scrapes regional news and events, classifying them via the HiveMind LLM to provide real-time semantic boosts for content relevant to the user's current location.
- **Predictive Warming**: Anticipates high-traffic scenarios and pre-warms the cache tiers to ensure zero-latency delivery during peak loads.

### 5. Server-Side Look-Ahead (Ghost Execution)

We eliminate client-side complex pre-warming logic. The engine automatically anticipates the user's next scroll based on `prewarm_lookahead` configuration and executes the next batch of rows in the background, caching them in Redis for zero-latency fetch.

### 6. Zero-Touch Contextual

We recognize devices and users passively.

- **Device Hash:** Deterministic fingerprinting using IP and User-Agent.
- **Visitor ID:** Transparent persistence via "Cookie-Lite" (Zero-Touch).
- **Identity Stitching:** Automatic merging of anonymous behavior into authenticated profiles upon login.

### 5. Decoupled Interface Standards (Netflix-Grade DTOs)

To ensure long-term maintainability and architectural integrity, Symphony 2.0 enforces **Parameter Consolidation** through dedicated Data Transfer Objects (DTOs).

- **Resilient Contracts**: Core methods no longer accept long lists of primitive arguments. Instead, they use specialized structs like `ScenarioExecutionContext` and `InteractionPayload`.
- **Extensibility**: New context parameters (e.g., location, network speed, experiment flags) can be added to DTOs without breaking internal API contracts.
- **Type Safety**: Deeply nested generic types are simplified via descriptive aliases (e.g., `PageLayoutCache`), reducing cognitive load for engineers.
- **Idiomatic Alignment**: All core system types implement standard Rust traits (`Default`, `FromStr`) for seamless ecosystem integration.

---

## 🚀 Architectural Deep-Dives

- **[👤 Identity & Stitching](./IDENTITY.md)**: How we track users without logins.
- **[⚡ Velocity Orchestration](./ORCHESTRATION.md)**: Parallel pipelining and Ghost execution logic.
- **[🎨 Page Composition](./PAGES.md)**: Smart layouts, SDUI metadata, and algorithmic ranking.
- **[🛡️ Resilience & Scale](./SECURITY.md)**: Circuit breakers, high-limit pools, and OTLP.

---

[🏠 Hub](../HUB.md) | [🔝 Top](#️-the-bongas-ai-symphony-20-core-architecture)
