# Architecture

## Overview

BONGAS-AI is a single-binary recommendation system built in Rust with ONNX-based ML inference. All recommendation scenarios are defined as JSONB pipelines in PostgreSQL, enabling hot-reload without restarts.

## System Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         BONGAS-AI BINARY                            │
│                                                                     │
│  ┌────────────────┐    ┌──────────────────┐   ┌─────────────────┐  │
│  │  BongasEngine  │───>│ Pipeline Executor│──>│  Scenario DB    │  │
│  │  Orchestrator  │    │  (JSONB->Stages) │   │  (PostgreSQL)   │  │
│  └────────┬───────┘    └────────┬─────────┘   └─────────────────┘  │
│           │                     │                                   │
│           v                     v                                   │
│  ┌────────────────┐    ┌──────────────────┐                        │
│  │ Staging Manager│    │  50+ Pipeline    │                        │
│  │  L2 Cache      │    │     Stages       │                        │
│  │ (PostgreSQL)   │    │ (Composable)     │                        │
│  └────────────────┘    └──────────────────┘                        │
│           │                     │                                   │
│           v                     v                                   │
│  ┌────────────────┐    ┌──────────────────┐   ┌─────────────────┐  │
│  │  Redis L1      │    │  ONNX Runtime    │──>│  ML Models      │  │
│  │  Cache         │    │  (Native Rust)   │   │  (.onnx files)  │  │
│  └────────────────┘    └──────────────────┘   └─────────────────┘  │
│                                 │                                   │
│                                 v                                   │
│                         ┌──────────────────┐                        │
│                         │  Feature Store   │                        │
│                         │  (Hourly Worker) │                        │
│                         └─────────┬────────┘                        │
│                                   │                                 │
└───────────────────────────────────┼─────────────────────────────────┘
                                    │
                  ┌─────────────────┴─────────────────┐
                  v                                   v
         ┌────────────────┐                   ┌────────────────┐
         │   ClickHouse   │                   │     Kafka      │
         │   (Analytics)  │                   │  (4 Consumers) │
         └────────────────┘                   └────────────────┘
```

## Core Components

### BongasEngine (`src/engine/`)
Central orchestrator that routes requests through the pipeline system with two-tier caching.

### Pipeline Executor (`src/scenarios/dynamic/`)
Executes JSONB-defined pipeline stages in sequence. Each stage implements the `PipelineStage` trait.

### ML Layer (`src/ml/`)
ONNX Runtime (Rust-native) for model inference. Models are loaded from `.onnx` files at startup.

### Caching (`src/cache/`)
- **L1**: Redis with 5-minute TTL (~70% hit rate)
- **L2**: PostgreSQL staging with 1-hour TTL (~25% hit rate)
- **Miss**: Full pipeline execution (~5%)

### Staleness Engine (`src/engine/staleness_engine.rs`)
Behavior-aware cache invalidation. Watches for user events (watch, like, new content) and selectively invalidates relevant cached scenarios.

### Security (`src/security/`)
8-layer security system: license validation, hardware fingerprinting, binary integrity, anti-debug, analysis tool detection, heartbeat, time expiration, network revocation.

## Data Flow

1. Request arrives at API endpoint
2. BongasEngine checks L1 cache (Redis)
3. On miss, checks L2 cache (PostgreSQL staging)
4. On miss, loads scenario pipeline from DB
5. Pipeline executor runs stages sequentially
6. ONNX Runtime performs ML inference where needed
7. Results cached in L2 then L1
8. Response returned

## Technology Stack

| Component | Technology |
|-----------|-----------|
| Runtime | Rust (Tokio async) |
| Web Framework | Axum |
| Database | PostgreSQL (sqlx) |
| Cache | Redis |
| Analytics | ClickHouse |
| Events | Apache Kafka (rdkafka) |
| ML Inference | ONNX Runtime (ort) |
| ML Training | Python/PyTorch (dev only) |
| Bandits | Native Rust (nalgebra, rand_distr) |
