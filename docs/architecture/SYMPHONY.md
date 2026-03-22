# 🏗️ The Bongas-AI Symphony 2.0: Core Architecture

[🏠 Hub](../HUB.md) | [👤 Identity](./IDENTITY.md) | [⚡ Streaming](./ORCHESTRATION.md) | [🎨 Pages](./PAGES.md) | [🔒 Security](./SECURITY.md)

---

## 🎼 The Paradigm: Server-Side Anticipatory Engine

In Symphony 2.0, we have evolved beyond simple Server-Driven UI (SDUI). Bongas-AI is now an **Anticipatory Engine** that minimizes user-perceived latency through server-side look-ahead and parallel fan-out.

**Bongas-AI flips the script.** The server is the Orchestrator, and the client is the Canvas.

👉 **[View the Interactive Data-Flow Architecture Diagram](./SYMPHONY_INTERACTIVE_DIAGRAM.html)**
> ⚠️ **GitLab/GitHub Notice:** By default, repository platforms render `.html` files as raw source code for security reasons. To view the animations and interactive elements, please **download** the `SYMPHONY_INTERACTIVE_DIAGRAM.html` file to your computer and open it in your web browser.

```mermaid
graph TD
    %% 1. Command & Control (Security Layer)
    subgraph Control["🛡️ Sovereign Control"]
        Server["🛰️ Central Server"] <---> Guard["🛡️ Sovereign Guard (Native Rust)"]
    end

    %% 2. User Interface
    subgraph UI_Space["User Interface"]
        direction LR
        UI["📱 Clients (Web/Mobile/TV)"]
        Admin["🔐 Admin Portal (Strategy Control)"]
    end
    UI_Space <---> Gateway["🌍 Stage API (Symphony Gateway)"]

    %% 3. Unified Engine Core (Rust)
    subgraph BongasAI["Bongas-AI Unified Engine (Rust)"]
        Gateway <---> Resolver["🎼 Symphony Resolver"]
        
        %% Security & Heartbeat
        Guard <---> Heartbeat["💓 Sovereign Heartbeat"]
        Guard --- Updater["🔄 Silent Updater"]

        %% Ingestion (Sensory)
        Ingestion["📥 Ingestion Processor"]

        %% Pillar grouping
        subgraph ReadPath["🎯 THE STAGE (Read Path)"]
            direction TB
            Inference["⚡ Candle Engine"]
            Cache["Ghost Cache"]
            Search["🔍 Tantivy Search"]
        end

        subgraph LearnPath["🏗️ THE BACKSTAGE (Learning Path)"]
            direction TB
            NativeTrainer["Native Rust Trainer"]
            TrainingState["TrainingState"]
        end

        subgraph Workers["Intelligence Workers (The Pulse)"]
            direction TB
            Sight["👁️ Sovereign Sight (Vision DNA)"]
            Tribe["👥 Tribe Orch (Clustering)"]
            Ghost["👻 Ghost Exec (Look-Ahead)"]
            Pulse["🌍 Regional Pulse (Hive Mind)"]
            SearchSync["🔄 Search Sync (Tantivy)"]
            Sound["👂 Sound Listener"]
            Decay["📉 Signal Decay"]
            Fatigue["🥱 Fatigue Sync"]
            Reasoning["🧠 Reasoning"]
            Digest["📝 Digest Worker"]
        end
    end

    %% 4. Sovereign Infrastructure (Horizontal)
    subgraph Infra["💾 Sovereign Infrastructure"]
        direction LR
        PG[(PostgreSQL)] --- RD[(Redis)] --- CH[(ClickHouse DNA)]
    end

    %% --- DATA & PROCESS FLOW ---
    
    %% 1. Synchronous Read Path (User Request)
    UI_Space -- "1. Request" --> Gateway
    Gateway -- "2. Orchestrate" --> Resolver
    Resolver -- "3. Query" --> Search
    Resolver -- "4. Fetch" --> Cache
    
    %% 2. Live Inference Flow
    TrainingState -- "5. Live Weights" --> Inference
    Resolver -- "6. Predict" --> Inference
    
    %% 3. The Pulse: Asynchronous Data Generation (Workers)
    SearchSync -- "Pulls Catalog" --> PG
    SearchSync -- "Updates Index" --> Search
    
    Ghost -- "Look-Ahead Data" --> RD
    Pulse -- "Semantic Vectors" --> RD
    Tribe -- "Cluster IDs" --> RD
    
    %% 4. The Sovereign Learning Loop (Secure Ingestion)
    %% a) Data Collection via Secure Gateway
    UI_Space -- "Telemetry & Clicks" --> Gateway
    Gateway -- "Buffer & Validate" --> Ingestion
    Ingestion -- "Sanitized Interactions" --> CH
    
    %% b) DNA Extraction
    TrainingState -- "Vision Weights" --> Sight
    Sight -- "Visual DNA & Audits" --> CH
    
    %% c) On-Premise Training
    CH -- "Interaction + Vision DNA" --> NativeTrainer
    NativeTrainer -- "Atomic Checkpoints" --> TrainingState

    %% Security & Management
    Guard -- "Silent Hot-Swap" --> BongasAI
    Cache <---> RD
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

### 7. Sovereign Intelligence: Symphony 3.0 (The Unified Pillar)

Bongas-AI achieves **100% Data Sovereignty** by moving model evolution and engine orchestration directly into the client's VPC. Symphony 3.0 provides a true "Single Binary" experience where all intelligence and security are native to the Rust core.

- **Native Rust Training (Candle):** We utilize the `candle-core` framework to run backpropagation and model adaptation directly in the core binary. This eliminates all Python-Bridge dependencies and external ML runtimes.
- **Integrated Sovereign Guard:** Security management, including binary integrity checks, mTLS heartbeats, and silent hot-swapping, is implemented as internal Rust modules. This ensures the binary protects itself without requiring external sidecar processes.
- **The Student Heads:** Massive foundation models remain read-only for feature extraction, while lightweight "Student Heads" (`VisionAuditorHead`, `StudentRankingHead`) are trained locally on private interaction DNA.
- **Atomic Weight Registry (TrainingState):** A high-concurrency, `RwLock`-guarded state manages live model weights. It enables **Atomic Weight Swaps**, where a newly trained model is hot-swapped into the inference path with zero downtime.
- **Production-Grade Persistence:** Background model saving uses a sync-to-async bridge (`spawn_blocking`) to safely write `.safetensors` checkpoints without blocking the request path.

---

## 🚀 Architectural Deep-Dives

- **[👤 Identity & Stitching](./IDENTITY.md)**: How we track users without logins.
- **[⚡ Velocity Orchestration](./ORCHESTRATION.md)**: Parallel pipelining and Ghost execution logic.
- **[🎨 Page Composition](./PAGES.md)**: Smart layouts, SDUI metadata, and algorithmic ranking.
- **[🛡️ Resilience & Scale](./SECURITY.md)**: Circuit breakers, high-limit pools, and OTLP.
- **[🔍 Embedded Search](../../src/search/README.md)**: The internal mechanics of the Tantivy search pillar.

---

[🏠 Hub](../HUB.md) | [🔝 Top](#️-the-bongas-ai-symphony-20-core-architecture)
