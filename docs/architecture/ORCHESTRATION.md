# ⚡ Streaming & Orchestration: The Velocity Engine

[🏠 Hub](../HUB.md) | [🏗️ Architecture](./SYMPHONY.md) | [👤 Identity](./IDENTITY.md) | [🎨 Pages](./PAGES.md)

---

## 🏎️ The Goal: Zero Perceived Latency

A world-class recommendation engine cannot wait for its slowest model. Bongas-AI uses **Parallel Pipelined Execution** to ensure that the user sees content as fast as the fastest component can deliver it.

## 🔄 Execution Models: Sequential vs. Parallel

### The Legacy Way (Sequential)

Each row waits for the previous one. A single slow ML model blocks the entire page.

- **Total Time:** Row 1 + Row 2 + Row 3...
- **User Experience:** "Stuttering" load.

### The Bongas-AI Way (Parallel Pipelined)

We trigger all rows simultaneously and stream them in a buffered order.

- **Total Time:** Max(Fastest Rows) + Overhead.
- **User Experience:** "Instant-On" pop.

```mermaid
graph TD
    A[Page Request] --> B[Fetch Page Layout]
    B --> C[Navigation & Manifest Event]
    C --> D[Parallel Dispatch]
    
    subgraph Execution Pool
        E1[Scenario 1: Continue Watching]
        E2[Scenario 2: ML Personalization]
        E3[Scenario 3: Trending Now]
    end

    D --> E1
    D --> E2
    D --> E3

    E1 --> F[SSE Stream]
    E3 --> F
    E2 --(Slow)--> F
```

## 🛠️ The "Velocity" Contract

1. **The Manifest Event**: The first event in the SSE stream tells the client exactly how many rows to expect and which rows to **pre-warm**.
2. **Buffer Unordered**: We execute scenarios in parallel (Concurrency: 5).
3. **Ordered Streaming**: While execution is parallel, we maintain a logical "order of importance" where possible, but never allow one slow row to kill the stream.
4. **Graceful Degradation**: If a scenario exceeds its timeout, the orchestrator emits an **Empty Comment** or a **Fallback Event** (e.g., "Popular") so the UI stays intact.

## 🛡️ The Shield: Resilience & Scale

To ensure "Netflix-Grade" reliability, Bongas-AI implements three layers of protection:

### 1. Fallback Scenarios (Fail-Over)

Admins can configure a `fallback_slug` for every row. If the primary (personalized) scenario fails, the engine automatically swaps it for a generic, high-performance alternative (e.g., "Trending").

### 2. Adaptive Rate Limiting

The SSE pool is protected by a visitor-level concurrency tracker. Each `visitor_id` is limited to **3 concurrent connections**, preventing device malfunctions or bot attacks from exhausting server resources.

### 3. Server-Side Look-Ahead (Ghost Execution)

Instead of relying on the client to trigger pre-warming, the engine performs **Anticipatory Execution**. When a user requests a batch of rows, the orchestrator automatically spawns a background "Ghost" task to execute the *next* batch of rows.

- **Zero-Latency Fetch**: Results are stored in a high-speed Redis "Ghost Cache" with a 5-minute TTL.
- **Immediate Response**: When the user scrolls and the client requests the next batch, the server streams the pre-computed results instantly.
- **Device-Awareness**: The look-ahead depth is dynamically adjusted based on the user's `device_type` (e.g., deeper for TV, shallower for Mobile).

---

## 🚀 Next Steps

- Learn how [**Smart Pages**](./PAGES.md) define these layouts.
- View the [**API Reference contract**](../api/REFERENCE.md).

---

[🏠 Hub](../HUB.md) | [🔝 Top](#-streaming--orchestration-the-velocity-engine)
