# 🏎️ Bongas-AI Performance Benchmarks

This directory contains the performance validation suite for the Bongas-AI Symphony engine. We use [Criterion.rs](https://github.com/bheisler/criterion.rs) for high-precision, statistically significant benchmarking.

---

## 📊 Benchmark Suites

### 1. ⚡ [Concurrency & Fan-Out](./concurrency_bench.rs)

Validates the performance of parallel scenario execution.

- **Goal:** Prove that executing 5 scenarios in parallel is significantly faster than sequential execution.
- **Metric:** Latency vs. Fan-out Factor.

### 2. 🛡️ [Thunder Killer (Thundering Herd)](./thunder_killer_bench.rs)

Tests the request consolidation logic.

- **Goal:** Ensure that 100 identical simultaneous requests for the same scenario only trigger 1 backend execution.
- **Metric:** Cache hit ratio and backend load under pressure.

### 3. 👻 [Engine & Ghost Execution](./engine_bench.rs)

Benchmarks the core `BongasEngine` and background pre-warming latency.

- **Goal:** Measure the overhead of server-side look-ahead.

### 4. 📦 [Pipeline & Logic](./pipeline_bench.rs)

Focuses on the performance of the transformation and ranking pipelines.

- **Goal:** Optimize the throughput of item filtering and sorting.

### 5. 💾 [Cache & Redis](./cache_bench.rs)

Measures the latency of the multi-tier (L1 LRU + L2 Redis) system.

- **Metric:** Time-to-fetch for various payload sizes.

---

## 🚀 Running Benchmarks

Ensure you have a local Redis instance running if testing L2 cache.

```bash
# Run all benchmarks
cargo bench

# Run a specific suite
cargo bench --bench concurrency_bench

# Run with specific filters
cargo bench -- "parallel_fanout"
```

## 📈 Performance Targets

| Component | Target Latency (p95) | Target Throughput |
| :--- | :--- | :--- |
| **Genesis SSE (First Row)** | < 100ms | 5,000 req/s |
| **Scenario Execution** | < 200ms | 2,000 req/s |
| **Cache Fetch (L1)** | < 1ms | 100,000 req/s |
| **Identity Middleware** | < 5ms | 10,000 req/s |

---

[🏠 Hub](../docs/HUB.md) | [🏗️ Architecture](../docs/architecture/SYMPHONY.md)
