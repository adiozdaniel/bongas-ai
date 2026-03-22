# 🏗️ The Bongas-AI Symphony 2.0: Core Architecture

[🏠 Hub](../HUB.md) | [👤 Identity](./IDENTITY.md) | [⚡ Streaming](./ORCHESTRATION.md) | [🎨 Pages](./PAGES.md) | [🔒 Security](./SECURITY.md)

---

## 🎼 The Paradigm: Server-Side Anticipatory Engine

In Symphony 2.0, we have evolved beyond simple Server-Driven UI (SDUI). Bongas-AI is now an **Anticipatory Engine** that minimizes user-perceived latency through server-side look-ahead and parallel fan-out.

**Bongas-AI flips the script.** The server is the Orchestrator, and the client is the Canvas.

👉 **[View the Interactive Data-Flow Architecture Diagram](./SYMPHONY_INTERACTIVE_DIAGRAM.html)**
> ⚠️ **GitLab/GitHub Notice:** By default, repository platforms render `.html` files as raw source code for security reasons. To view the animations and interactive elements, please **download** the `SYMPHONY_INTERACTIVE_DIAGRAM.html` file to your computer and open it in your web browser.

```mermaid
graph LR
    %% External Actors & Central Control
    subgraph ClientSpace ["External Actors & Central Command"]
        UI["Clients (Web/Mobile/TV)<br/>SDUI Canvas"]
        Admin["Admin Strategy Control"]
        CentralServer["🛰️ Bongas-Server (Cloud)<br/>Keys | Hyperparams | Binary Updates"]
    end

    %% Sovereign Intelligence Orchestrator
    subgraph SovereignOrchestrator ["🛡️ Training Pillar (Symphony 3.0)"]
        direction TB
        Heartbeat["Nightly Heartbeat<br/>(Runtime Decryption Keys)"]
        
        subgraph NativeTrainer ["Native Rust Training (Candle)"]
            VisionAuditor["Vision Auditor Head<br/>(Safety & Vibe)"]
            TribeRanker["Tribe Conductor Head<br/>(Ranking / Matrix Factorization)"]
            TrainingState["TrainingState<br/>(Atomic Weight Registry)"]
        end
        
        Persistence["Production Persistence<br/>(Safe safetensors saving)"]
    end

    %% Bongas-AI Application
    subgraph BongasAI ["Bongas-AI Core Engine (Rust)"]
        
        subgraph APIGateway ["Symphony Gateway"]
            StageAPI["🌍 THE STAGE"]
            BackstageAPI["🔐 THE BACKSTAGE"]
        end

        Resolver["Symphony Resolver"]

        subgraph RuntimePlane ["Runtime (Read Path)"]
            InferenceEngine["⚡ Candle Inference Engine<br/>(Hybrid Inference)"]
            TantivyIndex["🔍 Embedded Index<br/>(Tantivy)"]
            GhostCache["Ghost Cache<br/>(L1/L2 Redis)"]
        end
        
        subgraph WorkerPlane ["Intelligence (Write Path)"]
            SovereignSight["Sovereign Sight Worker<br/>(Visual DNA Extraction)"]
            TribeOrch["Tribe Orchestrator"]
            SearchSync["Index Sync"]
        end

        Notifier["Notification Dispatcher"]
    end

    %% Data Storage Layer
    subgraph DataLayer ["Sovereign Infrastructure"]
        PG[(PostgreSQL)]
        Redis[(Redis)]
        ClickHouse[(ClickHouse Interaction DNA)]
    end

    %% ML / Intelligence Integration
    subgraph IntelligenceLayer ["ML Assets"]
        FrozenModels["📦 Frozen Base Models<br/>(1.3B+ Params / Encrypted)"]
        StudentHeads["🧠 Student Heads<br/>(vision_head / ranking_head)"]
    end

    %% --- CONNECTIONS ---
    
    %% Orchestration & Security
    CentralServer <--> Heartbeat
    Heartbeat -- "Fetch Keys" --> FrozenModels
    
    %% Training Flow
    ClickHouse -- "Interaction DNA" --> NativeTrainer
    NativeTrainer --> TrainingState
    TrainingState -- "Atomic Swap" --> InferenceEngine
    TrainingState --> Persistence
    Persistence --> StudentHeads
    
    %% Discovery & Search
    UI <--> StageAPI
    StageAPI <--> Resolver
    Resolver <--> InferenceEngine
    Resolver <--> TantivyIndex
    
    %% DNA Extraction
    SovereignSight <--> FrozenModels
    SovereignSight -- "Extract DNA" --> ClickHouse
    SovereignSight -- "Audit" --> NativeTrainer

    %% Cache & Persistence
    InferenceEngine <--> GhostCache
    GhostCache <--> Redis
```

## 🌟 Key Architectural Pillars

### 1. Velocity Engine (Parallel Fan-Out)

Symphony 2.0 eliminates sequential bottlenecks. Using Rust's `futures` ecosystem, we execute up to 5 scenarios simultaneously per request.

- **Ordered Pipelining:** Uses `.buffered(5)` for SSE to maintain UI layout integrity.
- **Unordered Ghosting:** Uses `.buffer_unordered(5)` for background pre-warming to maximize throughput.

### 2. Sovereign Search & Deep Content (Milestone 20)

Unlike standard engines that rely on external search servers, Bongas-AI embeds its search logic directly into the core binary using **Tantivy**.

- **Embedded Indexing**: Zero network latency; search results are retrieved directly from memory-mapped disk files.
- **Sound Listener Intelligence**: A specialized worker extracts spoken words from video DNA, enabling users to search for content by what was *said*, not just the title.
- **Sheng-Native Analysis**: Customized linguistic tokenization using `jieba-rs` to correctly index and match Swahili and Sheng dialects.
- **Relevance Fusion**: Hybrid ranking that fuses BM25 keyword matching with Vision DNA vector similarity and real-time user history.

### 3. The OTLP Shield (High-Performance Observability)

Every request is protected and tracked by a "Shield" of distributed tracing, optimized for high throughput.

- **Trace ID Propagation**: Follows a request from the initial HTTP header, through the SSE fan-out, down to Postgres and Redis.
- **Identity Enrichment**: Spans are automatically enriched with `visitor_id`, `device_hash`, and `profile_id`.
- **Netflix-Scale Batching**: Telemetry is buffered and exported in configurable batches (`batch_size`, `max_queue_size`) to minimize the impact on the engine's performance.

### 4. Intelligence & Background Coordination (The Pulse)

The engine's "Intelligence" is not just reactive; it is proactive. Background workers constantly refine the data used by the discovery pipeline.

- **WorkersManager**: Centralized orchestration for all background maintenance tasks and intelligence workers.
- **Behavioral Tribes (TribeOrchestrator)**: Periodically clusters user profiles into behavioral "tribes" using K-Means clustering on embeddings.
- **Hyper-Local Semantic Pulse (RegionalPulseWorker)**: Scrapes regional news and events, classifying them via the HiveMind LLM to provide real-time semantic boosts.
- **Predictive Warming**: Anticipates high-traffic scenarios and pre-warm the cache tiers to ensure zero-latency delivery during peak loads.

### 5. Server-Side Look-Ahead (Ghost Execution)

We eliminate client-side complex pre-warming logic. The engine automatically anticipates the user's next scroll based on `prewarm_lookahead` configuration and executes the next batch of rows in the background, caching them in Redis for zero-latency fetch.

### 6. Zero-Touch Contextual Identity

We recognize devices and users passively to ensure privacy-first tracking.

- **Device Hash:** Deterministic fingerprinting using IP and User-Agent.
- **Visitor ID:** Transparent persistence via "Cookie-Lite" (Zero-Touch).
- **Identity Stitching:** Automatic merging of anonymous behavior into authenticated profiles upon login.

### 7. Sovereign Intelligence: Symphony 3.0 (The Training Pillar)

Bongas-AI achieves **100% Data Sovereignty** by moving model evolution directly into the client's VPC. Symphony 3.0 replaces external Python dependencies with a native Rust training ecosystem.

- **Native Rust Training (Candle):** We utilize the `candle-core` framework to run backpropagation and model adaptation directly in the core binary. This eliminates the "Python-Bridge" latency and security overhead.
- **The Student Heads:** Massive foundation models (`Frozen Senses`) remain read-only for feature extraction, while lightweight "Student Heads" (`VisionAuditorHead`, `StudentRankingHead`) are trained locally on private interaction DNA.
- **Atomic Weight Registry (TrainingState):** A high-concurrency, `RwLock`-guarded state manages live model weights. It enables **Atomic Weight Swaps**, where a newly trained model is hot-swapped into the inference path with zero downtime.
- **Production-Grade Persistence:** Background model saving uses a sync-to-async bridge (`spawn_blocking`) to safely write `.safetensors` checkpoints without blocking the request path.
- **Hybrid Inference:** The engine executes a sub-millisecond forward pass by fusing pre-extracted content DNA from ClickHouse with the live, locally-evolved Student Head weights.
- **Sovereign Sight Worker:** A specialized intelligence worker that orchestrates visual DNA extraction and maturity forensic auditing using the live Vision Auditor student head.

---

## 🚀 Architectural Deep-Dives

- **[👤 Identity & Stitching](./IDENTITY.md)**: How we track users without logins.
- **[⚡ Velocity Orchestration](./ORCHESTRATION.md)**: Parallel pipelining and Ghost execution logic.
- **[🎨 Page Composition](./PAGES.md)**: Smart layouts, SDUI metadata, and algorithmic ranking.
- **[🛡️ Resilience & Scale](./SECURITY.md)**: Circuit breakers, high-limit pools, and OTLP.
- **[🔍 Embedded Search](../../src/search/README.md)**: The internal mechanics of the Tantivy search pillar.

---

[🏠 Hub](../HUB.md) | [🔝 Top](#️-the-bongas-ai-symphony-20-core-architecture)
