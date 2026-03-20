# ⚡ Streaming & Orchestration: The Velocity Engine 2.0

[🏠 Hub](../HUB.md) | [🏗️ Architecture](./SYMPHONY.md) | [👤 Identity](./IDENTITY.md) | [🎨 Pages](./PAGES.md)

---

## 🏎️ The Goal: Zero Perceived Latency

A world-class recommendation engine cannot wait for its slowest model. Symphony 2.0 uses **Concurrent Fan-Out** to ensure that the user sees content as fast as the fastest component can deliver it.

## 🔄 Concurrent Execution: The Fan-Out Model

In Symphony 2.0, we have completely eradicated sequential loops. Using Rust's `futures` and `tokio` runtime, we implement a high-concurrency fan-out model.

### 1. Ordered SSE Streaming (Live)

For the user's active viewport, we use `.buffered(5)`.

- **Parallel Execution:** Up to 5 scenarios fire simultaneously.
- **Ordered Delivery:** Rows are emitted to the client in the exact order specified by the Admin Page Layout (e.g., Hero → Continue Watching → Trending).
- **Impact:** The fastest rows start rendering immediately, and slow rows only delay themselves, not the entire page.

### 2. Unordered Ghost Execution (Background)

For server-side look-ahead, we use `.buffer_unordered(5)`.

- **Max Throughput:** We don't care about the order when writing to the cache.
- **Background Persistence:** Results are aggregated and written atomically to Redis.

```mermaid
graph TD
    A[Page Request] --> B[Fetch Page Layout]
    B --> C[Navigation & Manifest Event]
    C --> D[Parallel Dispatch: Fan-Out 5]
    
    subgraph Execution Pool (Tokio)
        E1[Scenario 1: Hero]
        E2[Scenario 2: ML Picks]
        E3[Scenario 3: Trending]
    end

    D --> E1
    D --> E2
    D --> E3

    E1 --> F[SSE Stream: buffered]
    E2 --> F
    E3 --> F
```

## 👻 Ghost Execution: Server-Side Anticipation

We eliminate client-side complex pre-warming logic. The engine automatically anticipates the user's next scroll or search intent.

### 1. Paginated Look-Ahead
- **Trigger:** Every `genesis` or paginated request triggers a background task for the *next* batch.
- **The "Ghost Cache":** Results are stored in Redis with a key format: `ghost:user_{id}:page_{slug}:offset_{offset}`.
- **TTL:** 5 minutes (300s).
- **Concurrency:** Uses `buffer_unordered` to maximize pre-warming speed without blocking the main request thread.

### 2. Ghost Search (Predictive Querying)
- **Trigger:** Active keystroke events from the client.
- **Logic:** The engine performs a low-latency "Reflex Search" against the **Embedded Tantivy Index** before the user even hits Enter.
- **Impact:** Populates the UI search context with "zero-wait" relevant items.

## 💓 The Pulse: Background Orchestration

While the request-handling layer is high-performance and reactive, the engine's long-term intelligence is maintained by **The Pulse**—a coordinated suite of background workers managed by the `WorkersManager`.

### 1. Centralized Lifecycle Management

All background workers respond to the global engine shutdown signal and run in dedicated tokio tasks to ensure zero impact on request latency.

- **TribeOrchestrator**: Periodically clusters user profiles into behavioral "tribes" using K-Means clustering on embeddings. These tribes are cached in Redis for high-speed lookup during content recovery.
- **SoundListenerWorker**: Monitors the content ledger for unindexed videos. It extracts spoken words and pushes them to ClickHouse (Forensics) and the **Embedded Index (Search)**.
- **SearchSyncWorker**: Performs a **Differential Census** between Postgres metadata and the local Tantivy index to ensure 100% search consistency without external dependencies.
- **RegionalPulseWorker**: Scrapes and vectorizes regional news and events. It populates Redis with "Semantic Pulses" that the ranking layer uses to boost content relevant to the user's location.
- **FatigueSynchronizer**: Pluggable state-synchronizer that tracks item exposures across the cluster, ensuring that "Content Fatigue" logic is always based on the most recent interaction data.

### 2. Synchronization Strategy

Workers primarily communicate with the request path via **Redis** or the **Embedded Search Index**. This creates a clean separation of concerns:
- **Write-Path (Workers):** Perform heavy computation or I/O-intensive scraping and write the refined intelligence to Redis or Tantivy.
- **Read-Path (Pipeline):** Perform sub-millisecond lookups from Redis or memory-resident index files to apply intelligence to recommendations.

## 🎻 The Middleware Symphony

Every request passes through a coordinated stack of global middlewares before reaching the Orchestrator. This ensures that the engine only processes valid, safe, and traceable traffic.

### 1. The Global Pipeline Stack

1.  **Identity Shield**: passive fingerprinting and context extraction.
2.  **Adaptive Rate Limiter**: Multi-tier (L1/L2) protection against scrapers.
3.  **Platform Security**: Signature and key validation for trusted clients.
4.  **Bulkhead (Global)**: Enforces hard concurrency limits on the entire API surface.
5.  **Circuit Breaker (Global)**: Trips on high error rates to protect downstream pools.
6.  **OTLP Instrumented**: End-to-end tracing injection.

## 🛡️ Resilience & Scale

### 1. Connection Pool Scaling

To handle the 5x concurrency multiplier (1 request = 5 concurrent DB/Redis checkouts), we have hardened our infrastructure:

- **Postgres (sqlx):** Scaled to **100** max connections.
- **Redis:** Scaled to **50** pool size.
- **Aggressive Timeouts:** `connection_timeout` (5s) and `request_timeout` (10s) ensure we fail-fast rather than stalling.

### 2. Row-Level Resilience

Every scenario execution is wrapped in a circuit breaker. If a specific scenario fails or times out:

- It emits an SSE comment or an empty fallback row.
- The rest of the parallel stream continues unaffected.

### 3. Graceful Shutdown

The orchestration engine supports a "The Finale" shutdown sequence:

1. Signal background workers to stop.
2. Wait for active fan-outs to complete.
3. Flush all OTLP trace spans.
4. Close all connection pools explicitly.

---

## 🚀 Next Steps

- Learn how [**Smart Pages**](./PAGES.md) define these layouts.
- View the [**API Reference contract**](../api/REFERENCE.md).

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-streaming--orchestration-the-velocity-engine-20)
