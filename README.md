# ![Bongas-AI Logo](./docs/logo.svg) Bongas-AI Symphony 2.0: Content Intelligence Platform

## *Unified Content Intelligence & Sovereign Discovery*

**Bongas-AI Symphony 2.0** is a high-scale, AI-powered engine for search and personalization. Built natively in **Rust** 🦀, it delivers zero-latency discovery while replacing fragmented SaaS tools with a single, sovereign intelligence pillar.

-----

## 🏗️ The Velocity Architecture

We decouple the **Read-Path (The Stage)** from the **Write-Path (The Backstage)** to ensure ML training never throttles user experience.

* **⚡ Concurrent Fan-Out:** Streams "Hero" and "Trending" rows via **SSE** for instant rendering.
* **👻 Ghost Execution:** Predictive background caching in Redis eliminates loading spinners.
* **🎭 Dual-Plane Control:** Runtime delivery is isolated from heavy **Candle** model training.

-----

## 🏭 Training Factory: [Bongas-ML](https://github.com/adiozdaniel/bongas-ml)

The backbone of the "Backstage" write-path. **Bongas-ML** handles heavy-lift model distillation, feature engineering, and `.safetensors` quantization, ensuring that only optimized, production-ready weights reach the Symphony runtime.

-----

## 🧠 Core Intelligence Domains

| Domain | Mechanism | Impact |
| :--- | :--- | :--- |
| **Discovery** | `TribeOrchestrator` | K-Means clustering for "Lookalike" behavioral targeting. |
| **Refinement** | `FatigueSync` | Exponential penalties for over-exposed content via Redis. |
| **Sovereignty** | `Candle` / `Rust` | Local training & inference via `.safetensors`—no data exfiltration. |
| **Explanation** | `Semantic Why` | LLM-powered reasoning to build user trust (e.g., *"Because you like..."*). |

-----

## 🚀 Strengths

* **🔍 Embedded Hybrid Search:** Sub-10ms full-text matching using **Tantivy**.
* **🧠 The Conductor:** Agentic reasoning for system config via local **SLMs**.
* **🌍 Linguistic Brain:** Culturally aware fine-tuning for regional contexts.

-----

## 🛠️ The Stack

* **Engine:** `Rust` (`tokio`, `axum`)
* **State:** `Redis` (Hot) / `PostgreSQL` (Record) / `ClickHouse` (Telemetry)
* **AI:** `Candle` & `HiveMind` (Inference) / [**Bongas-ML**](https://github.com/adiozdaniel/bongas-ml) (Training)

-----

### 📖 [**Visit the Documentation Hub**](docs/HUB.md)

*Built with 🦀 for uncompromising speed and safety.*
